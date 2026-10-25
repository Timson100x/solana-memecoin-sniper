use crate::types::{Position, RiskDecision};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

const MAX_DAILY_LOSS_DEFAULT: f64 = 0.5;

/// Hard risk-layer limits.
#[derive(Debug, Clone)]
pub struct Limits {
    pub max_daily_loss_sol: Decimal,
    pub max_trade_sol: Decimal,
    pub max_open_positions: u32,
    pub max_slippage_bps: u16,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_daily_loss_sol: Decimal::try_from(MAX_DAILY_LOSS_DEFAULT).unwrap(),
            max_trade_sol: Decimal::try_from(0.05).unwrap(),
            max_open_positions: 5,
            max_slippage_bps: 400,
        }
    }
}

/// Evaluate a candidate trade against hard limits.
pub fn evaluate_trade(
    limits: &Limits,
    daily_pnl: Decimal,
    trade_sol: Decimal,
    open_position_count: u32,
    dry_run: bool,
) -> RiskDecision {
    if dry_run {
        return RiskDecision::Reject("dry_run mode".to_string());
    }

    if daily_pnl <= -limits.max_daily_loss_sol {
        return RiskDecision::Reject("daily loss limit exceeded".to_string());
    }

    if trade_sol > limits.max_trade_sol {
        return RiskDecision::Reject("trade exceeds MAX_TRADE_SOL".to_string());
    }

    if open_position_count >= limits.max_open_positions {
        return RiskDecision::Reject("max open positions reached".to_string());
    }

    RiskDecision::Allow
}

/// Evaluate an open position for forced close.
pub fn evaluate_position(limits: &Limits, position: &Position, current_pnl: Decimal) -> RiskDecision {
    let _ = dec!(0.0);
    if current_pnl <= -limits.max_trade_sol {
        return RiskDecision::ForceClose("stop loss".to_string());
    }
    RiskDecision::Allow
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn dry_run_rejects() {
        let l = Limits::default();
        assert_eq!(
            evaluate_trade(&l, dec!(0.0), dec!(0.01), 0, true),
            RiskDecision::Reject("dry_run mode".to_string())
        );
    }

    #[test]
    fn daily_loss_blocks() {
        let l = Limits::default();
        assert!(matches!(
            evaluate_trade(&l, dec!(-0.6), dec!(0.01), 0, false),
            RiskDecision::Reject(_)
        ));
    }

    #[test]
    fn open_positions_blocks() {
        let l = Limits::default();
        assert!(matches!(
            evaluate_trade(&l, dec!(0.0), dec!(0.01), 5, false),
            RiskDecision::Reject(_)
        ));
    }

    #[test]
    fn allow_trade() {
        let l = Limits::default();
        assert_eq!(
            evaluate_trade(&l, dec!(0.0), dec!(0.01), 0, false),
            RiskDecision::Allow
        );
    }
}
