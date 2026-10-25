use crate::types::{Position, RiskDecision};
use rust_decimal::Decimal;
use tracing::warn;

/// Trait for pluggable trading strategies.
pub trait Strategy: Send + Sync {
    fn name(&self) -> &'static str;
    fn evaluate(&self, ctx: &StrategyContext) -> Option<Vec<crate::types::Signal>>;
}

/// Neutral context passed to strategies.
pub struct StrategyContext;

/// A no-op strategy that emits no signals — safe placeholder.
#[derive(Default)]
pub struct NeutralStrategy;

impl Strategy for NeutralStrategy {
    fn name(&self) -> &'static str {
        "neutral"
    }

    fn evaluate(&self, _ctx: &StrategyContext) -> Option<Vec<crate::types::Signal>> {
        None
    }
}

/// Risk wrapper that consults the hard limits before returning a decision.
pub fn guard(
    limits: &crate::risk::Limits,
    daily_pnl: Decimal,
    trade_sol: Decimal,
    open_positions: u32,
    positions: &[Position],
    dry_run: bool,
) -> RiskDecision {
    for pos in positions {
        let _ = pos;
    }
    if let RiskDecision::Reject(reason) =
        crate::risk::evaluate_trade(limits, daily_pnl, trade_sol, open_positions, dry_run)
    {
        return RiskDecision::Reject(reason);
    }
    RiskDecision::Allow
}

#[allow(dead_code)]
pub fn warn_if_forced(_reason: &str) {
    warn!("{}", _reason);
}
