use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SwapEvent {
    pub timestamp: DateTime<Utc>,
    pub sol_amount: f64,
    pub is_buy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPosition {
    pub mint: String,
    pub entry_price_sol: f64,
    pub amount_tokens: f64,
    pub sol_spent: f64,
    pub opened_at: DateTime<Utc>,
    pub take_profit_pct: f64,
    pub stop_loss_pct: f64,
    pub max_hold_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraduatedToken {
    pub mint: String,
    pub pool: Option<String>,
    pub graduated_at: DateTime<Utc>,
    pub last_volume_5m: f64,
    pub last_volume_1m: f64,
    pub last_buy_ratio: f64,
    pub score: f64,
    pub last_updated: DateTime<Utc>,
    pub last_swap_at: DateTime<Utc>,
    #[serde(skip)]
    pub recent_swaps: VecDeque<SwapEvent>,
}

impl GraduatedToken {
    pub fn new(mint: String, pool: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            mint,
            pool,
            graduated_at: now,
            last_volume_5m: 0.0,
            last_volume_1m: 0.0,
            last_buy_ratio: 0.0,
            score: 0.0,
            last_updated: now,
            last_swap_at: now,
            recent_swaps: VecDeque::with_capacity(512),
        }
    }

    pub fn age_hours(&self) -> f64 {
        Utc::now()
            .signed_duration_since(self.graduated_at)
            .num_seconds() as f64
            / 3600.0
    }

    pub fn minutes_since_last_swap(&self) -> f64 {
        Utc::now()
            .signed_duration_since(self.last_swap_at)
            .num_seconds() as f64
            / 60.0
    }

    pub fn push_swap(&mut self, sol_amount: f64, is_buy: bool) {
        let now = Utc::now();
        self.recent_swaps.push_back(SwapEvent {
            timestamp: now,
            sol_amount,
            is_buy,
        });
        self.last_swap_at = now;
        self.prune_old_swaps(now);
    }

    fn prune_old_swaps(&mut self, now: DateTime<Utc>) {
        let cutoff = now - chrono::Duration::minutes(5);
        while let Some(front) = self.recent_swaps.front() {
            if front.timestamp < cutoff {
                self.recent_swaps.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn recalculate_metrics(&mut self) {
        let now = Utc::now();
        self.prune_old_swaps(now);

        let cutoff_1m = now - chrono::Duration::minutes(1);
        let mut vol_5m = 0.0;
        let mut vol_1m = 0.0;
        let mut buy_vol = 0.0;
        let mut sell_vol = 0.0;

        for swap in &self.recent_swaps {
            vol_5m += swap.sol_amount;
            if swap.is_buy {
                buy_vol += swap.sol_amount;
            } else {
                sell_vol += swap.sol_amount;
            }
            if swap.timestamp >= cutoff_1m {
                vol_1m += swap.sol_amount;
            }
        }

        self.last_volume_5m = vol_5m;
        self.last_volume_1m = vol_1m;
        let total = buy_vol + sell_vol;
        self.last_buy_ratio = if total > 0.0 { buy_vol / total } else { 0.5 };
        self.last_updated = now;
    }
}

pub struct RiskState {
    pub daily_pnl_sol: Mutex<f64>,
    pub live_trading_enabled: AtomicBool,
    pub open_positions: Mutex<Vec<OpenPosition>>,
}

impl RiskState {
    pub fn new(live_enabled: bool) -> Self {
        Self {
            daily_pnl_sol: Mutex::new(0.0),
            live_trading_enabled: AtomicBool::new(live_enabled),
            open_positions: Mutex::new(Vec::new()),
        }
    }

    pub fn is_live(&self) -> bool {
        self.live_trading_enabled.load(Ordering::Relaxed)
    }

    pub fn disable_live(&self) {
        self.live_trading_enabled.store(false, Ordering::Relaxed);
    }

    pub fn record_pnl(&self, delta_sol: f64) {
        let mut pnl = self.daily_pnl_sol.lock();
        *pnl += delta_sol;
    }

    pub fn daily_pnl(&self) -> f64 {
        *self.daily_pnl_sol.lock()
    }
}

pub struct AppState {
    pub watchlist: DashMap<String, GraduatedToken>,
    pub risk: RiskState,
}

impl AppState {
    pub fn new(live_enabled: bool) -> Self {
        Self {
            watchlist: DashMap::new(),
            risk: RiskState::new(live_enabled),
        }
    }
}

pub type AppSharedState = Arc<AppState>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JupiterQuoteResponse {
    pub input_mint: String,
    pub in_amount: String,
    pub output_mint: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: u16,
    pub price_impact_pct: String,
    pub route_plan: serde_json::Value,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    pub swap_transaction: String,
    pub last_valid_block_height: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TradeLogEntry {
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub mint: String,
    pub score: f64,
    pub sol_amount: f64,
    pub signature: Option<String>,
    pub reason: Option<String>,
}
