use anyhow::Result;
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

mod helius;
mod jupiter;
mod risk;
mod scoring;
mod types;

use types::{AppState, AppSharedState, RiskManager};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .or_else(|_| "info,pump_runner_bot=debug".parse()),
        )
        .init();

    info!("Starting Pump Runner Bot v0.3.0...");

    let live_trading_env = std::env::var("ENABLE_LIVE_TRADING")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let max_loss_sol: f64 = std::env::var("MAX_DAILY_LOSS_SOL")
        .unwrap_or_else(|_| "0.5".to_string())
        .parse()
        .unwrap_or(0.5);

    let max_loss_lamports = (max_loss_sol * 1_000_000_000.0) as u64;
    let max_concentration: f64 = std::env::var("MAX_TOP10_HOLDER_CONCENTRATION")
        .unwrap_or_else(|_| "0.45".to_string())
        .parse()
        .unwrap_or(0.45);

    let state: AppSharedState = Arc::new(AppState {
        watchlist: DashMap::new(),
        risk_manager: RiskManager {
            live_trading_enabled: AtomicBool::new(live_trading_env),
            daily_loss_lamports: AtomicU64::new(0),
            max_daily_loss_lamports: max_loss_lamports,
            max_top10_concentration: max_concentration,
        },
    });

    let state_grad = state.clone();
    tokio::spawn(async move {
        if let Err(e) = helius::listen_graduations(state_grad).await {
            error!("Fataler Fehler im Graduation Listener: {:?}", e);
        }
    });

    let state_scan = state.clone();
    tokio::spawn(async move {
        if let Err(e) = helius::scan_volume_and_score(state_scan).await {
            error!("Fataler Fehler im Volume Scanner: {:?}", e);
        }
    });

    info!("Bot v0.3.0 aktiv. Ueberwache Graduationen, Risiken & High-Momentum Runner...");

    signal::ctrl_c().await?;
    info!("Shutdown Signal empfangen. Beende Bot sauber...");

    Ok(())
}
" filePath="/workspace/app/src/main.rs