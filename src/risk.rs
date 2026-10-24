use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use tracing::{info, warn};

use crate::types::{AppSharedState, TradeLogEntry};

pub async fn check_rug_shield(mint: &str) -> Result<bool> {
    let max_pct: f64 = env::var("MAX_TOP10_HOLDERER_PCT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(45.0);

    let rpc_url = env::var("HELIUS_RPC_URL").context("HELIUS_RPC_URL missing")?;
    let client = Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenLargestAccounts",
        "params": [mint]
    });

    let resp = client
        .post(&rpc_url)
        .json(&body)
        .send()
        .await
        .context("getTokenLargestAccounts failed")?;

    let val: serde_json::Value = resp.json().await?;
    let accounts = val
        .pointer("/result/value")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if accounts.is_empty() {
        warn!("RugShield: no holder data for {}", mint);
        return Ok(false);
    }

    let mut top10_amount = 0.0_f64;
    let mut total_supply = 0.0_f64;
    for (i, acc) in accounts.iter().enumerate() {
        let amount = acc
            .pointer("/uiAmount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if i < 10 {
            top10_amount += amount;
        }
        total_supply += amount;
    }
    if total_supply <= 0.0 {
        total_supply = top10_amount * 1.5;
    }
    let pct = if total_supply > 0.0 {
        (top10_amount / total_supply) * 100.0
    } else {
        100.0
    };

    info!(
        "RugShield | mint={} | top10={:.1}% (limit={:.1}%)",
        mint, pct, max_pct
    );
    Ok(pct <= max_pct)
}

pub fn check_circuit_breaker(state: &AppSharedState) -> bool {
    let max_loss: f64 = env::var("MAX_DAILY_LOSS_SOL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.5);
    let pnl = state.risk.daily_pnl();
    if pnl <= -max_loss {
        if state.risk.is_live() {
            warn!(
                "CIRCUIT BREAKER triggered | daily_pnl={:.3} SOL | limit=-{:.3}",
                pnl, max_loss
            );
            state.risk.disable_live();
        }
        return false;
    }
    true
}

pub fn log_trade(entry: &TradeLogEntry) {
    let path = env::var("TRADE_LOG_PATH").unwrap_or_else(|_| "./trade_log.jsonl".to_string());
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        if let Ok(line) = serde_json::to_string(entry) {
            let _ = writeln!(file, "{}", line);
        }
    }
}

pub fn positions_to_close(state: &AppSharedState) -> Vec<(String, String)> {
    let max_hold: u64 = env::var("MAX_HOLD_MINUTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(45);
    let now = Utc::now();
    let mut to_close = Vec::new();
    let positions = state.risk.open_positions.lock();
    for pos in positions.iter() {
        let hold_mins = now
            .signed_duration_since(pos.opened_at)
            .num_minutes()
            .max(0) as u64;
        if hold_mins >= max_hold {
            to_close.push((pos.mint.clone(), "MAX_HOLD".to_string()));
        }
    }
    to_close
}

pub fn register_position(state: &AppSharedState, mint: &str, sol_spent: f64, tokens: f64) {
    let tp: f64 = env::var("TAKE_PROFIT_PCT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50.0);
    let sl: f64 = env::var("STOP_LOSS_PCT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20.0);
    let max_hold: u64 = env::var("MAX_HOLD_MINUTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(45);

    let entry_price = if tokens > 0.0 {
        sol_spent / tokens
    } else {
        0.0
    };
    let pos = crate::types::OpenPosition {
        mint: mint.to_string(),
        entry_price_sol: entry_price,
        amount_tokens: tokens,
        sol_spent,
        opened_at: Utc::now(),
        take_profit_pct: tp,
        stop_loss_pct: sl,
        max_hold_minutes: max_hold,
    };
    state.risk.open_positions.lock().push(pos);
    info!("Position registered | mint={} | sol={:.4}", mint, sol_spent);
}
