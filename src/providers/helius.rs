use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{debug, error, info, warn};

const PUMP_FUN_PROGRAM: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
const PUMPSWAP_PROGRAM: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";

/// State handle placeholder kept generic so this module compiles when unused.
pub type AppSharedState = ();

/// Reconnect-capable Helius WebSocket listener.
pub async fn listen_graduations(_state: AppSharedState) -> Result<()> {
    let ws_url =
        std::env::var("HELIUS_WS_URL").context("HELIUS_WS_URL must be set")?;
    let mut reconnect_delay = Duration::from_secs(2);

    loop {
        info!("Connecting to Helius WebSocket...");
        match run_ws_session(&ws_url).await {
            Ok(_) => {
                warn!("WS session ended. Reconnecting...");
                reconnect_delay = Duration::from_secs(2);
            }
            Err(e) => {
                error!("WS error: {:?}. Reconnect in {:?}", e, reconnect_delay);
                sleep(reconnect_delay).await;
                reconnect_delay = (reconnect_delay * 2).min(Duration::from_secs(60));
            }
        }
    }
}

async fn run_ws_session(ws_url: &str) -> Result<()> {
    let (ws_stream, _) = connect_async(ws_url)
        .await
        .context("Helius WS connect failed")?;
    let (mut write, mut read) = ws_stream.split();

    let subscribe_msg = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "transactionSubscribe",
        "params": [
            { "accountInclude": [PUMP_FUN_PROGRAM, PUMPSWAP_PROGRAM] },
            { "commitment": "confirmed", "encoding": "jsonParsed", "maxSupportedTransactionVersion": 0 }
        ]
    });

    write
        .send(Message::Text(subscribe_msg.to_string()))
        .await
        .context("Failed to send subscription")?;
    info!("Subscribed to Pump.fun + PumpSwap events");

    while let Some(msg) = read.next().await {
        let msg = msg.context("WS receive error")?;
        match msg {
            Message::Text(text) => {
                debug!("tx notification: {}", &text[..text.len().min(120)]);
            }
            Message::Ping(data) => {
                let _ = write.send(Message::Pong(data)).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    Ok(())
}
