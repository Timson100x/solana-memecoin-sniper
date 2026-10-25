use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{error, info, warn};

mod config;
mod execution;
mod observability;
mod providers {
    pub mod helius;
    pub mod jupiter;
}
mod risk;
mod strategy;
mod types;

use config::Config;
use execution::{ExecutionMode, Executor};
use observability::init_logging;

/// Solana momentum trading bot — paper-first.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Validate configuration and exit (no secrets required).
    #[arg(long)]
    config_check: bool,

    /// Force paper / dry-run mode (default).
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    paper: bool,

    /// Enable live trading (requires EXECUTION_CONFIRMATION and wallet).
    #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
    live: bool,

    /// Explicit risk confirmation string required for live mode.
    #[arg(long)]
    execution_confirmation: Option<String>,

    /// Path to .env file.
    #[arg(long, value_name = "FILE")]
    env_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.env_file {
        Some(p) => dotenvy::from_path(p).ok(),
        None => dotenvy::dotenv().ok(),
    }
    .ok();

    init_logging();

    let cfg = Config::from_env().context("Failed to load configuration")?;

    if cli.config_check {
        info!("Configuration check complete.");
        cfg.print_summary();
        if !cfg.has_all_secrets() {
            warn!("Optional secrets missing — providers will remain dormant.");
        }
        return Ok(());
    }

    if cli.live {
        let confirmation = cli.execution_confirmation.clone().unwrap_or_default();
        if !cfg.live_trading_enabled || confirmation != "I_UNDERSTAND_THE_RISK" {
            error!(
                "Live trading requires ENABLE_LIVE_TRADING=true AND \
                 --execution-confirmation 'I_UNDERSTAND_THE_RISK'"
            );
            return Err(anyhow::anyhow!("Live trading confirmation missing"));
        }
        if cfg.wallet_private_key.is_empty() {
            error!("Live trading requires SOLANA_WALLET_PRIVATE_KEY.");
            return Err(anyhow::anyhow!("Wallet private key missing"));
        }
        info!("⚠️ LIVE trading enabled. Trades are irreversible.");
    }

    let mode = if cli.live {
        ExecutionMode::Live
    } else {
        ExecutionMode::Paper
    };

    info!("Starting bot in {:?} mode", mode);
    let _executor: Box<dyn execution::Executor> =
        execution::build_executor(mode, &cfg).await?;

    info!("Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await.context("ctrl_c")?;
    info!("Shutting down.");
    Ok(())
}
