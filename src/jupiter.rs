use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::VersionedTransaction,
};
use std::env;
use std::str::FromStr;
use tracing::{info, warn};

use crate::types::{JupiterQuoteResponse, SwapResponse};

const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

const JITO_TIP_ACCOUNTS: &[&str] = &[
    "96gYZGLnJYVFmbzsm8qK5k3DnIFVVuVcHqYgP1kFaF8",
    "HFqU5x63VTqvQss8hp11i4bVmkYTgzG2xTmbuuB6yE5",
    "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
    "ADaUMid9yfUytqMBgoptGy4xBeoEvGhT6ku4Sg2vYB1",
    "DfXygSm4jCyNCybVYYK6DwvZTpoKySBZQu5ysx85ta2P",
    "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
    "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
    "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
];

pub struct JupiterClient {
    http: Client,
    api_url: String,
    rpc: RpcClient,
}

impl JupiterClient {
    pub fn new() -> Result<Self> {
        let api_url = env::var("JUPITER_API_URL")
            .unwrap_or_else(|_| "https://quote-api.jup.ag/v6".to_string());
        let rpc_url = env::var("HELIUS_RPC_URL").context("HELIUS_RPC_URL missing")?;
        Ok(Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(25))
                .build()?,
            api_url,
            rpc: RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed()),
        })
    }

    pub async fn get_buy_quote(
        &self,
        token_mint: &str,
        sol_lamports: u64,
        slippage_bps: u16,
    ) -> Result<JupiterQuoteResponse> {
        self.get_quote(SOL_MINT, token_mint, sol_lamports, slippage_bps)
            .await
    }

    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount_lamports: u64,
        slippage_bps: u16,
    ) -> Result<JupiterQuoteResponse> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&onlyDirectRoutes=false",
            self.api_url, input_mint, output_mint, amount_lamports, slippage_bps
        );
        let resp = self.http.get(&url).send().await.context("quote request")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Jupiter quote {}: {}", status, body));
        }
        let quote: JupiterQuoteResponse = resp.json().await.context("parse quote")?;
        info!(
            "Quote | in={} -> out={} | impact={}%",
            quote.in_amount, quote.out_amount, quote.price_impact_pct
        );
        Ok(quote)
    }

    pub async fn execute_buy(
        &self,
        token_mint: &str,
        sol_lamports: u64,
        slippage_bps: u16,
        keypair: &Keypair,
    ) -> Result<String> {
        let quote = self
            .get_buy_quote(token_mint, sol_lamports, slippage_bps)
            .await?;
        let quote_value = serde_json::to_value(&quote)?;
        let priority = env::var("PRIORITY_FEE_LAMPORTS").unwrap_or_else(|_| "auto".to_string());

        let payload = json!({
            "quoteResponse": quote_value,
            "userPublicKey": keypair.pubkey().to_string(),
            "wrapAndUnwrapSol": true,
            "dynamicComputeUnitLimit": true,
            "prioritizationFeeLamports": priority
        });

        let swap_url = format!("{}/swap", self.api_url);
        let resp = self
            .http
            .post(&swap_url)
            .json(&payload)
            .send()
            .await
            .context("Jupiter /swap")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Jupiter swap {}: {}", status, body));
        }

        let swap_resp: SwapResponse = resp.json().await.context("parse swap")?;
        let tx_bytes = STANDARD
            .decode(&swap_resp.swap_transaction)
            .context("base64 decode")?;
        let mut versioned_tx: VersionedTransaction =
            bincode::deserialize(&tx_bytes).context("deserialize tx")?;

        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        versioned_tx.message.set_recent_blockhash(recent_blockhash);
        versioned_tx.sign(&[keypair], recent_blockhash);

        let use_jito = env::var("USE_JITO").unwrap_or_else(|_| "true".to_string()) == "true";
        if use_jito {
            match self.send_jito_bundle(&versioned_tx, keypair).await {
                Ok(sig) => {
                    info!("Jito bundle landed | {}", sig);
                    return Ok(sig);
                }
                Err(e) => warn!("Jito failed ({:?}), falling back to Helius RPC", e),
            }
        }

        info!("Sending via Helius RPC...");
        let signature = self
            .rpc
            .send_and_confirm_transaction(&versioned_tx)
            .await
            .context("Helius send failed")?;
        info!("Helius landed | {}", signature);
        Ok(signature.to_string())
    }

    async fn send_jito_bundle(
        &self,
        tx: &VersionedTransaction,
        keypair: &Keypair,
    ) -> Result<String> {
        let engine = env::var("JITO_BLOCK_ENGINE_URL")
            .unwrap_or_else(|_| "https://mainnet.block-engine.jito.wtf".to_string());
        let tip_lamports: u64 = env::var("JITO_TIP_LAMPORTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10_000);
        let tip_account = JITO_TIP_ACCOUNTS
            .get((Utc::now().timestamp() as usize) % JITO_TIP_ACCOUNTS.len())
            .unwrap_or(&JITO_TIP_ACCOUNTS[0]);
        let tip_pubkey = Pubkey::from_str(tip_account)?;
        let _tip_ix = system_instruction::transfer(&keypair.pubkey(), &tip_pubkey, tip_lamports);

        let tx_b64 = STANDARD.encode(bincode::serialize(tx)?);
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendBundle",
            "params": [[tx_b64]]
        });

        let url = format!("{}/api/v1/bundles", engine);
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Jito sendBundle")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Jito error {}: {}", status, text));
        }
        let val: serde_json::Value = resp.json().await?;
        let bundle_id = val
            .get("result")
            .and_then(|r| r.as_str())
            .unwrap_or("unknown")
            .to_string();
        Ok(format!("jito_bundle_{}", bundle_id))
    }
}

pub async fn try_buy_token(mint: &str) -> Result<String> {
    let pk_str = env::var("PRIVATE_KEY").context("PRIVATE_KEY not set")?;
    let keypair = Keypair::from_base58_string(&pk_str);

    let sol_amount: f64 = env::var("BUY_AMOUNT_SOL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.05);
    let sol_lamports = (sol_amount * 1_000_000_000.0) as u64;
    let slippage: u16 = env::var("SLIPPAGE_BPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);

    let client = JupiterClient::new()?;
    client
        .execute_buy(mint, sol_lamports, slippage, &keypair)
        .await
}
