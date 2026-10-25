use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use tracing::info;

use crate::types::Quote;

/// Jupiter/Metis quote client. No swap execution here — only quoting.
pub struct JupiterClient {
    http: Client,
    api_url: String,
}

impl JupiterClient {
    pub fn new(api_url: String) -> Self {
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(25))
                .build()
                .expect("reqwest client"),
            api_url,
        }
    }

    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount_lamports: u64,
        slippage_bps: u16,
    ) -> Result<Quote> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&onlyDirectRoutes=false",
            self.api_url, input_mint, output_mint, amount_lamports, slippage_bps
        );

        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .context("Jupiter quote request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Jupiter quote {} : {}", status, body);
        }

        let _ = json!({"url": url});
        let quote: Quote = resp.json().await.context("parse quote")?;
        info!(
            "Quote | in={} out={} impact={}%",
            quote.in_amount, quote.out_amount, quote.price_impact_pct
        );
        Ok(quote)
    }

    /// Optional authenticated endpoint support placeholder.
    pub async fn ping(&self) -> Result<()> {
        let _ = self
            .http
            .get(format!("{}/trade", self.api_url))
            .send()
            .await?;
        Ok(())
    }
}
