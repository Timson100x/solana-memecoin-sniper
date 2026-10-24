use anyhow::{Context, Result};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{debug, error, info, warn};

use crate::risk::{check_circuit_breaker, check_rug_shield, log_trade, register_position};
use crate::scoring::calculate_score;
use crate::types::{AppSharedState, GraduatedToken, TradeLogEntry};

const PUMP_FUN_PROGRAM: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
const PUMPSWAP_PROGRAM: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

pub async fn listen_graduations(state: AppSharedState) -> Result<()> {
    let ws_url =
        std::env::var("HELIUS_WS_URL").context("HELIUS_WS_URL muss in .env gesetzt sein")?;
    let mut reconnect_delay = Duration::from_secs(2);

    loop {
        info!("Verbinde mit Helius WebSocket...");
        match run_ws_session(&ws_url, state.clone()).await {
            Ok(_) => {
                warn!("WS Session ended. Reconnect...");
                reconnect_delay = Duration::from_secs(2);
            }
            Err(e) => {
                error!("WS Error: {:?}. Reconnect in {:?}", e, reconnect_delay);
                sleep(reconnect_delay).await;
                reconnect_delay = (reconnect_delay * 2).min(Duration::from_secs(60));
            }
        }
    }
}

async fn run_ws_session(ws_url: &str, state: AppSharedState) -> Result<()> {
    let (ws_stream, _) = connect_async(ws_url)
        .await
        .context("Helius WS connect failed")?;
    let (mut write, mut read) = ws_stream.split();

    let subscribe_msg = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "transactionSubscribe",
        "params": [
            {
                "accountInclude": [PUMP_FUN_PROGRAM, PUMPSWAP_PROGRAM],
                "failed": false,
                "vote": false
            },
            {
                "commitment": "confirmed",
                "encoding": "jsonParsed",
                "transactionDetails": "full",
                "showRewards": false,
                "maxSupportedTransactionVersion": 0
            }
        ]
    });

    write
        .send(Message::Text(subscribe_msg.to_string()))
        .await
        .context("Failed to send subscription")?;
    info!("Subscribed to Pump.fun + PumpSwap");

    while let Some(msg) = read.next().await {
        let msg = msg.context("WS receive error")?;
        match msg {
            Message::Text(text) => {
                if let Err(e) = handle_message(&text, &state).await {
                    debug!("handle_message: {:?}", e);
                }
            }
            Message::Ping(data) => {
                let _ = write.send(Message::Pong(data)).await;
            }
            Message::Close(_) => {
                warn!("Close frame");
                break;
            }
            _ => {}
        }
    }
    Ok(())
}

async fn handle_message(text: &str, state: &AppSharedState) -> Result<()> {
    let json_val: serde_json::Value = serde_json::from_str(text)?;
    if json_val.get("method").and_then(|m| m.as_str()) != Some("transactionNotification") {
        return Ok(());
    }

    if let Some(mint) = extract_graduation_mint(&json_val) {
        if !state.watchlist.contains_key(&mint) {
            let max_size: usize = std::env::var("MAX_WATCHLIST_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500);
            if state.watchlist.len() >= max_size {
                warn!("Watchlist full ({}), skipping {}", max_size, mint);
            } else {
                info!("NEW GRADUATED TOKEN: {}", mint);
                state
                    .watchlist
                    .insert(mint.clone(), GraduatedToken::new(mint, None));
            }
        }
    }

    if let Some((mint, sol_amount, is_buy)) = extract_swap_info(&json_val) {
        if let Some(mut entry) = state.watchlist.get_mut(&mint) {
            entry.push_swap(sol_amount, is_buy);
        }
    }

    Ok(())
}

fn extract_graduation_mint(tx_json: &serde_json::Value) -> Option<String> {
    let logs = tx_json
        .pointer("/params/result/transaction/meta/logMessages")?
        .as_array()?;
    let is_migrate = logs.iter().any(|log| {
        let s = log.as_str().unwrap_or("").to_lowercase();
        s.contains("migrate") || s.contains("graduat") || s.contains("complete")
    });
    if !is_migrate {
        return None;
    }
    if let Some(balances) = tx_json.pointer("/params/result/transaction/meta/postTokenBalances") {
        if let Some(arr) = balances.as_array() {
            for bal in arr {
                if let Some(mint) = bal.get("mint").and_then(|m| m.as_str()) {
                    if mint != WSOL_MINT && mint.len() >= 32 {
                        return Some(mint.to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_swap_info(tx_json: &serde_json::Value) -> Option<(String, f64, bool)> {
    let meta = tx_json.pointer("/params/result/transaction/meta")?;
    let pre = meta.get("preTokenBalances")?.as_array()?;
    let post = meta.get("postTokenBalances")?.as_array()?;
    let mut sol_delta = 0.0_f64;
    let mut token_mint: Option<String> = None;

    for (pre_bal, post_bal) in pre.iter().zip(post.iter()) {
        let mint = post_bal.get("mint")?.as_str()?;
        let pre_amt = pre_bal
            .pointer("/uiTokenAmount/uiAmount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let post_amt = post_bal
            .pointer("/uiTokenAmount/uiAmount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let delta = post_amt - pre_amt;
        if mint == WSOL_MINT {
            sol_delta += delta;
        } else if mint.len() >= 32 {
            token_mint = Some(mint.to_string());
        }
    }

    let mint = token_mint?;
    if sol_delta.abs() < 0.0001 {
        return None;
    }
    let is_buy = sol_delta < 0.0;
    Some((mint, sol_delta.abs(), is_buy))
}

pub async fn scan_volume_and_score(state: AppSharedState) -> Result<()> {
    let scan_interval: u64 = std::env::var("SCAN_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6);
    let threshold: f64 = std::env::var("SCORE_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(70.0);
    let min_vol: f64 = std::env::var("MIN_VOLUME_5M_SOL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(15.0);
    let inactive_mins: f64 = std::env::var("INACTIVE_TIMEOUT_MINS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(15.0);
    let min_score_keep: f64 = std::env::var("MIN_SCORE_TO_KEEP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10.0);

    info!(
        "Scanner started | interval={}s | threshold={} | cleanup inactive>{}m",
        scan_interval, threshold, inactive_mins
    );

    loop {
        sleep(Duration::from_secs(scan_interval)).await;

        let to_remove: Vec<String> = state
            .watchlist
            .iter()
            .filter(|e| {
                let t = e.value();
                t.minutes_since_last_swap() > inactive_mins || t.score < min_score_keep
            })
            .map(|e| e.key().clone())
            .collect();
        for mint in &to_remove {
            state.watchlist.remove(mint);
            debug!("Removed inactive/low-score token {}", mint);
        }

        let keys: Vec<String> = state.watchlist.iter().map(|e| e.key().clone()).collect();
        for mint in keys {
            if let Some(mut entry) = state.watchlist.get_mut(&mint) {
                entry.recalculate_metrics();
                let age = entry.age_hours();
                let score = calculate_score(
                    entry.last_volume_5m,
                    entry.last_volume_1m,
                    entry.last_buy_ratio,
                    age,
                );
                entry.score = score;
                if score >= threshold && entry.last_volume_5m >= min_vol {
                    info!(
                        "HIGH SCORE | mint={} | score={:.1} | vol5m={:.1} | buy={:.0}% | age={:.1}h",
                        mint, score, entry.last_volume_5m, entry.last_buy_ratio * 100.0, age
                    );

                    if !check_circuit_breaker(&state) {
                        log_trade(&TradeLogEntry {
                            timestamp: Utc::now(),
                            action: "SKIP_CIRCUIT".into(),
                            mint: mint.clone(),
                            score,
                            sol_amount: 0.0,
                            signature: None,
                            reason: Some("daily loss limit".into()),
                        });
                        continue;
                    }

                    match check_rug_shield(&mint).await {
                        Ok(true) => {
                            if state.risk.is_live() {
                                match crate::jupiter::try_buy_token(&mint).await {
                                    Ok(sig) => {
                                        let sol = std::env::var("BUY_AMOUNT_SOL")
                                            .ok()
                                            .and_then(|v| v.parse().ok())
                                            .unwrap_or(0.05);
                                        register_position(&state, &mint, sol, 0.0);
                                        log_trade(&TradeLogEntry {
                                            timestamp: Utc::now(),
                                            action: "BUY".into(),
                                            mint: mint.clone(),
                                            score,
                                            sol_amount: sol,
                                            signature: Some(sig),
                                            reason: None,
                                        });
                                    }
                                    Err(e) => error!("Buy failed {}: {:?}", mint, e),
                                }
                            } else {
                                log_trade(&TradeLogEntry {
                                    timestamp: Utc::now(),
                                    action: "DRY_RUN".into(),
                                    mint: mint.clone(),
                                    score,
                                    sol_amount: 0.0,
                                    signature: None,
                                    reason: Some("live trading disabled".into()),
                                });
                                info!("DRY-RUN would buy {}", mint);
                            }
                        }
                        Ok(false) => {
                            log_trade(&TradeLogEntry {
                                timestamp: Utc::now(),
                                action: "SKIP_RUG".into(),
                                mint: mint.clone(),
                                score,
                                sol_amount: 0.0,
                                signature: None,
                                reason: Some("top10 holder concentration".into()),
                            });
                            warn!("RugShield blocked {}", mint);
                        }
                        Err(e) => warn!("RugShield error for {}: {:?}", mint, e),
                    }
                }
            }
        }
    }
}
