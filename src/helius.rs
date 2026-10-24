use anyhow::{Context, Result};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};

use crate::scoring::calculate_score;
use crate::types::{AppSharedState, GraduatedToken};

const PUMP_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

pub async fn listen_graduations(state: AppSharedState) -> Result<()> {
    let ws_url = std::env::var("HELIUS_WS_URL")
        .context("HELIUS_WS_URL muss in .env gesetzt sein")?;

    loop {
        info!("Verbinde mit Helius WebSocket fuer Graduation-Events...");

        let (mut ws_stream, _) = match connect_async(&ws_url).await {
            Ok(val) => val,
            Err(e) => {
                error!("Fehler bei Verbindung zu Helius WS: {:?}. Reconnect in 5s...", e);
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        info!("Erfolgreich mit Helius WebSocket verbunden. Abonniere Pump.fun Program...");

        let subscribe_msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "transactionSubscribe",
            "params": [
                {
                    "accountInclude": [PUMP_PROGRAM_ID]
                },
                {
                    "commitment": "processed",
                    "encoding": "jsonParsed",
                    "maxSupportedTransactionVersion": 0
                }
            ]
        });

        if let Err(e) = ws_stream.send(Message::Text(subscribe_msg.to_string())).await {
            error!("Fehler beim Senden des Subscription-Payloads: {:?}", e);
            sleep(Duration::from_secs(3)).await;
            continue;
        }

        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(mint) = extract_graduation_mint(&json_val) {
                            if !state.watchlist.contains_key(&mint) {
                                info!("Neuer graduierter Token erkannt: {}", mint);
                                let entry = GraduatedToken {
                                    mint: mint.clone(),
                                    pool: None,
                                    graduated_at: Utc::now(),
                                    last_volume_5m: 0.0,
                                    last_volume_1m: 0.0,
                                    last_buy_ratio: 0.0,
                                    score: 0.0,
                                    last_updated: Utc::now(),
                                };
                                state.watchlist.insert(mint, entry);
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    warn!("WebSocket-Verbindung vom Server geschlossen. Reconnecting...");
                    break;
                }
                Err(e) => {
                    error!("WebSocket Fehler: {:?}. Reconnecting...", e);
                    break;
                }
                _ => {}
            }
        }

        sleep(Duration::from_secs(3)).await;
    }
}

fn extract_graduation_mint(tx_json: &serde_json::Value) -> Option<String> {
    let logs = tx_json.pointer("/params/result/value/transaction/meta/logMessages")?.as_array()?;

    let is_graduation = logs.iter().any(|log| {
        let s = log.as_str().unwrap_or("");
        s.contains("Instruction: InitializeMint2") || s.contains("complete")
    });

    if is_graduation {
        let account_keys = tx_json.pointer("/params/result/value/transaction/transaction/message/accountKeys")?.as_array()?;
        if let Some(first_key) = account_keys.first() {
            return first_key["pubkey"].as_str().map(|s| s.to_string());
        }
    }

    None
}

pub async fn scan_volume_and_score(state: AppSharedState) -> Result<()> {
    let scan_interval: u64 = std::env::var("SCAN_INTERVAL_SECS")
        .unwrap_or_else(|_| "6".to_string())
        .parse()
        .unwrap_or(6);

    let threshold: f64 = std::env::var("SCORE_THRESHOLD")
        .unwrap_or_else(|_| "70.0".to_string())
        .parse()
        .unwrap_or(70.0);

    loop {
        sleep(Duration::from_secs(scan_interval)).await;

        let now = Utc::now();

        state.watchlist.retain(|_, entry| {
            let inactive_duration = (now - entry.last_updated).num_minutes();
            inactive_duration < 15 && entry.score >= 10.0
        });

        for mut entry in state.watchlist.iter_mut() {
            let mint = entry.key().clone();
            let (vol_5m, vol_1m, buy_ratio) = simulate_market_data(&mint, now.timestamp());

            let age_hours = (now - entry.graduated_at).num_seconds() as f64 / 3600.0;
            let score = calculate_score(vol_5m, vol_1m, buy_ratio, age_hours);

            entry.last_volume_5m = vol_5m;
            entry.last_volume_1m = vol_1m;
            entry.last_buy_ratio = buy_ratio;
            entry.score = score;
            entry.last_updated = now;

            if score >= threshold {
                info!(
                    "HIGH SCORE SIGNAL | Mint: {} | Score: {:.1} | 5m Vol: {:.1} SOL | BuyRatio: {:.0}%",
                    mint, score, vol_5m, buy_ratio * 100.0
                );
            }
        }
    }
}

fn simulate_market_data(mint: &str, timestamp: i64) -> (f64, f64, f64) {
    let hash = mint.bytes().fold(0u64, |acc, b| acc.wrapping_add(b as u64));
    let seed = hash.wrapping_add((timestamp / 6) as u64);

    let vol_5m = ((seed % 100) as f64) + 5.0;
    let vol_1m = vol_5m * (((seed % 40) as f64 + 10.0) / 100.0);
    let buy_ratio = 0.50 + (((seed % 45) as f64) / 100.0);

    (vol_5m, vol_1m, buy_ratio)
}
" filePath="/workspace/app/src/helius.rs