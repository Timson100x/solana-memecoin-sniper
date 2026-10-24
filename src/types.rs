use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraduatedToken {
    pub mint: String,
    pub pool: Option<String>,
    pub graduated_at: DateTime<Utc>,
    pub last_volume_5m: f64,
    pub last_volume_1m: f64,
    pub last_buy_ratio: f64,
    pub score: f64,
    pub last_updated: DateTime<Utc>,
}

pub struct RiskManager {
    pub live_trading_enabled: AtomicBool,
    pub daily_loss_lamports: AtomicU64,
    pub max_daily_loss_lamports: u64,
    pub max_top10_concentration: f64,
}

pub struct AppState {
    pub watchlist: DashMap<String, GraduatedToken>,
    pub risk_manager: RiskManager,
}

pub type AppSharedState = Arc<AppState>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JupiterQuoteResponse {
    pub input_mint: String,
    pub in_amount: String,
    pub output_mint: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: u16,
    pub price_impact_pct: String,
    pub route_plan: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    pub user_public_key: String,
    pub quote_response: JupiterQuoteResponse,
    pub wrap_and_unwrap_sol: Option<bool>,
    pub dynamic_compute_unit_limit: Option<bool>,
    pub priority_fee_in_sol: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    pub swap_transaction: String,
    pub last_valid_block_height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeLogEntry {
    pub timestamp: DateTime<Utc>,
    pub mint: String,
    pub score: f64,
    pub action: String,
    pub tx_signature: String,
    pub pnl_sol: Option<f64>,
}
" filePath="/workspace/app/src/types.rs