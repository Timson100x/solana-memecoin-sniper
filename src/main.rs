use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

mod helius;
mod jupiter;
mod risk;
mod scoring;
mod types;

use types::{AppState, AppSharedState};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,pump_runner_bot=debug".parse().unwrap()),
        )
        .with_target(false)
        .init();

    let live = std::env::var("ENABLE_LIVE_TRADING").unwrap_or_default() == "true";
    info!("pump_runner_bot v0.3.0 starting");
    info!(
        "Features: real volume | RugShield | CircuitBreaker | Jito | Watchlist cleanup | Trade log"
    );
    info!(
        "Live trading: {}",
        if live { "ENABLED" } else { "DISABLED (dry-run)" }
    );

    let state: AppSharedState = Arc::new(AppState::new(live));

    let state_ws = state.clone();
    let ws_handle = tokio::spawn(async move {
        if let Err(e) = helius::listen_graduations(state_ws).await {
            error!("WebSocket fatal: {:?}", e);
        }
    });

    let state_scan = state.clone();
    let scan_handle = tokio::spawn(async move {
        if let Err(e) = helius::scan_volume_and_score(state_scan).await {
            error!("Scanner fatal: {:?}", e);
        }
    });

    info!("Bot running. Ctrl+C to stop.");

    signal::ctrl_c().await?;
    info!("Shutdown signal received");

    ws_handle.abort();
    scan_handle.abort();

    info!(
        "Final daily PnL: {:.4} SOL | Open positions: {}",
        state.risk.daily_pnl(),
        state.risk.open_positions.lock().len()
    );
    info!("pump_runner_bot stopped");
    Ok(())
}
