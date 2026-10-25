use anyhow::{bail, Result};
use std::env;

/// Application configuration derived from environment variables.
#[derive(Debug)]
pub struct Config {
    pub helius_api_key: String,
    pub helius_rpc_url: String,
    pub helius_ws_url: String,
    pub jupiter_api_url: String,
    pub wallet_private_key: String,
    pub live_trading_enabled: bool,
    pub dry_run: bool,
    pub buy_amount_sol: f64,
    pub slippage_bps: u16,
    pub priority_fee_lamports: u64,
    pub max_daily_loss_sol: f64,
    pub max_trade_sol: f64,
    pub max_open_positions: u32,
    pub max_hold_minutes: u64,
    pub trade_log_path: String,
    pub log_level: String,
    pub telegram_bot_token: String,
    pub telegram_chat_id: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            helius_api_key: String::new(),
            helius_rpc_url: String::new(),
            helius_ws_url: String::new(),
            jupiter_api_url: "https://quote-api.jup.ag/v6".to_string(),
            wallet_private_key: String::new(),
            live_trading_enabled: false,
            dry_run: true,
            buy_amount_sol: 0.05,
            slippage_bps: 400,
            priority_fee_lamports: 100000,
            max_daily_loss_sol: 0.5,
            max_trade_sol: 0.05,
            max_open_positions: 5,
            max_hold_minutes: 45,
            trade_log_path: "./logs/trade_log.jsonl".to_string(),
            log_level: "info".to_string(),
            telegram_bot_token: String::new(),
            telegram_chat_id: String::new(),
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        let mut cfg = Config::default();

        cfg.helius_api_key = env::var("HELIUS_API_KEY").unwrap_or_default();
        cfg.helius_rpc_url = env::var("HELIUS_RPC_URL").unwrap_or_default();
        cfg.helius_ws_url = env::var("HELIUS_WS_URL").unwrap_or_default();
        cfg.jupiter_api_url = env::var("JUPITER_API_URL")
            .unwrap_or_else(|_| "https://quote-api.jup.ag/v6".to_string());
        cfg.wallet_private_key = env::var("SOLANA_WALLET_PRIVATE_KEY").unwrap_or_default();

        cfg.live_trading_enabled = env::var("ENABLE_LIVE_TRADING")
            .map(|v| v == "true")
            .unwrap_or(false);
        cfg.dry_run = !cfg.live_trading_enabled;

        cfg.buy_amount_sol = parse_f64("BUY_AMOUNT_SOL", 0.05);
        cfg.slippage_bps = parse_u16("SLIPPAGE_BPS", 400);
        cfg.priority_fee_lamports = parse_u64("PRIORITY_FEE_LAMPORTS", 100_000);
        cfg.max_daily_loss_sol = parse_f64("MAX_DAILY_LOSS_SOL", 0.5);
        cfg.max_trade_sol = parse_f64("MAX_TRADE_SOL", 0.05);
        cfg.max_open_positions = parse_u32("MAX_OPEN_POSITIONS", 5);
        cfg.max_hold_minutes = parse_u64("MAX_HOLD_MINUTES", 45);
        cfg.trade_log_path = env::var("TRADE_LOG_PATH")
            .unwrap_or_else(|_| "./logs/trade_log.jsonl".to_string());
        cfg.log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        cfg.telegram_bot_token = env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
        cfg.telegram_chat_id = env::var("TELEGRAM_CHAT_ID").unwrap_or_default();

        Self::validate(&cfg)?;
        Ok(cfg)
    }

    pub fn has_all_secrets(&self) -> bool {
        !self.helius_api_key.is_empty() && !self.helius_rpc_url.is_empty() && !self.helius_ws_url.is_empty()
    }

    pub fn validate(&self) -> Result<()> {
        if self.max_trade_sol > self.buy_amount_sol + 1e-9 {
            bail!("MAX_TRADE_SOL cannot exceed BUY_AMOUNT_SOL");
        }
        if self.slippage_bps > 5000 {
            bail!("SLIPPAGE_BPS exceeds sane maximum (5000)");
        }
        if self.max_daily_loss_sol <= 0.0 {
            bail!("MAX_DAILY_LOSS_SOL must be positive");
        }
        Ok(())
    }

    pub fn print_summary(&self) {
        let wallet_masked = mask(&self.wallet_private_key);
        let api_masked = mask(&self.helius_api_key);
        tracing::info!(
            "mode: dry_run={}, live={}, wallet=[{}], helius_key=[{}]",
            self.dry_run,
            self.live_trading_enabled,
            wallet_masked,
            api_masked,
        );
    }
}

fn parse_f64(key: &str, default: f64) -> f64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn parse_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn parse_u32(key: &str, default: u32) -> u32 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn parse_u16(key: &str, default: u16) -> u16 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn mask(secret: &str) -> String {
    if secret.is_empty() {
        "<empty>".to_string()
    } else if secret.len() <= 4 {
        format!("*{secret}*")
    } else {
        format!("*{}...{}", &secret[..2], &secret[secret.len() - 2..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_positive_loss() {
        let mut cfg = Config::default();
        cfg.max_daily_loss_sol = 0.0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_invalid_slippage() {
        let mut cfg = Config::default();
        cfg.slippage_bps = 9999;
        assert!(cfg.validate().is_err());
    }
}
