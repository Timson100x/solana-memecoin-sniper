use anyhow::Result;
use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::Ordering;
use tracing::{info, warn};

use crate::types::{AppSharedState, TradeLogEntry};

pub struct RiskEngine;

impl RiskEngine {
    pub fn check_circuit_breaker(state: &AppSharedState) -> bool {
        let current_loss = state.risk_manager.daily_loss_lamports.load(Ordering::Relaxed);
        let max_loss = state.risk_manager.max_daily_loss_lamports;

        if current_loss >= max_loss {
            warn!("CIRCUIT BREAKER AUSGELOEST! Tagesverlust ({} lamports) ueberschreitet Limit ({}). Live-Trading blockiert.", current_loss, max_loss);
            state.risk_manager.live_trading_enabled.store(false, Ordering::Relaxed);
            false
        } else {
            true
        }
    }

    pub async fn verify_rug_shield(_mint: &str, max_concentration: f64) -> Result<bool> {
        let simulated_top10_share = 0.28;

        if simulated_top10_share > max_concentration {
            warn!("RugShield Alarm: Top-10 Holder besitzen {:.1}% (Limit: {:.1}%)", simulated_top10_share * 100.0, max_concentration * 100.0);
            Ok(false)
        } else {
            Ok(true)
        }
    }

    pub fn log_trade(entry: &TradeLogEntry) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("trade_log.jsonl")?;

        let json_line = serde_json::to_string(entry)?;
        writeln!(file, "{}", json_line)?;
        info!("Trade Log gespeichert: [{}] {} - Score: {:.1}", entry.action, entry.mint, entry.score);
        Ok(())
    }
}
" filePath="/workspace/app/src/risk.rs