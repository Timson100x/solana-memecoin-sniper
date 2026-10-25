use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A token mint observed on-chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}

/// A quote from a DEX aggregator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub slippage_bps: u16,
    pub price_impact_pct: String,
    pub route_plan: serde_json::Value,
}

/// Trading signal emitted by a strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub token: Token,
    pub side: Side,
    pub score: Decimal,
    pub confidence: Decimal,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

/// An open trading position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub token_mint: String,
    pub opened_at: DateTime<Utc>,
    pub entry_sol: Decimal,
    pub entry_price: Decimal,
    pub size_tokens: Decimal,
}

/// Decision returned by the risk layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskDecision {
    Allow,
    Reject(String),
    ForceClose(String),
}
