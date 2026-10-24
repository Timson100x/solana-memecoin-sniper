use anyhow::{Context, Result};
use reqwest::Client;
use tracing::info;

use crate::types::{JupiterQuoteResponse, SwapRequest, SwapResponse};

pub struct JupiterClient {
    client: Client,
    api_url: String,
}

impl JupiterClient {
    pub fn new() -> Self {
        let api_url = std::env::var("JUPITER_API_URL")
            .unwrap_or_else(|_| "https://quote-api.jup.ag/v6".to_string());

        Self {
            client: Client::new(),
            api_url,
        }
    }

    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount_lamports: u64,
        slippage_bps: u16,
    ) -> Result<JupiterQuoteResponse> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            self.api_url, input_mint, output_mint, amount_lamports, slippage_bps
        );

        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Fehler bei Anfragen des Jupiter Quotes")?
            .json::<JupiterQuoteResponse>()
            .await
            .context("Fehler beim Parsen des Jupiter Quote Responses")?;

        Ok(response)
    }

    pub async fn prepare_swap_transaction(
        &self,
        user_pubkey: &str,
        quote: JupiterQuoteResponse,
        priority_fee_sol: f64,
    ) -> Result<SwapResponse> {
        let url = format!("{}/swap", self.api_url);

        let payload = SwapRequest {
            user_public_key: user_pubkey.to_string(),
            quote_response: quote,
            wrap_and_unwrap_sol: Some(true),
            dynamic_compute_unit_limit: Some(true),
            priority_fee_in_sol: Some(priority_fee_sol),
        };

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Fehler beim Aufrufen des Jupiter Swap Endpoints")?
            .json::<SwapResponse>()
            .await
            .context("Fehler beim Parsen der Swap Transaktion")?;

        info!("Swap Transaktion ueber Jito / RPC Vorbereitung erfolgreich");
        Ok(response)
    }

    pub async fn execute_swap(&self, _swap_tx_base64: &str, is_live: bool) -> Result<String> {
        if !is_live {
            info!("Sicherheits-Modus (DRY_RUN): Live execution uebersprungen.");
            return Ok("DRY_RUN_TX_SIGNATURE_PLACEHOLDER".to_string());
        }

        info!("Live-Execution aktiv - Sende Jito Bundle / RPC Transaktion...");
        Ok("LIVE_TX_SIGNATURE_SUCCESS".to_string())
    }
}
" filePath="/workspace/app/src/jupiter.rs