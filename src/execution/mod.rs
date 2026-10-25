use crate::config::Config;
use anyhow::{bail, Result};
use tracing::info;

/// Execution mode resolved at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Paper,
    Live,
}

/// Executor trait. Live execution requires explicit opt-in at construction.
pub trait Executor: Send + Sync {
    fn mode(&self) -> ExecutionMode;
    fn mode_label(&self) -> &'static str {
        match self.mode() {
            ExecutionMode::Paper => "paper",
            ExecutionMode::Live => "live-dangerous",
        }
    }
}

/// Paper executor — the default. It never signs or sends transactions.
pub struct PaperExecutor;

impl Executor for PaperExecutor {
    fn mode(&self) -> ExecutionMode {
        ExecutionMode::Paper
    }
}

/// Live executor — constructed only if fully confirmed.
pub struct LiveExecutor {
    #[allow(dead_code)]
    wallet_private_key: String,
}

impl Executor for LiveExecutor {
    fn mode(&self) -> ExecutionMode {
        ExecutionMode::Live
    }
}

/// Factory with safety gates. Live mode requires both config flags.
pub async fn build_executor(mode: ExecutionMode, cfg: &Config) -> Result<Box<dyn Executor>> {
    match mode {
        ExecutionMode::Paper => {
            info!("PaperExecutor active — no transactions will be signed.");
            Ok(Box::new(PaperExecutor))
        }
        ExecutionMode::Live => {
            if !cfg.live_trading_enabled {
                bail!("ENABLE_LIVE_TRADING must be true");
            }
            if cfg.wallet_private_key.is_empty() {
                bail!("SOLANA_WALLET_PRIVATE_KEY must be set for live trading");
            }
            info!("LiveExecutor active — LIVE TRADING ON.");
            Ok(Box::new(LiveExecutor {
                wallet_private_key: cfg.wallet_private_key.clone(),
            }))
        }
    }
}
