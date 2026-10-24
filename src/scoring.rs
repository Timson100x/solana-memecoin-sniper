use tracing::debug;

pub fn calculate_score(
    volume_5m_sol: f64,
    volume_1m_sol: f64,
    buy_ratio: f64,
    age_hours: f64,
) -> f64 {
    let mut score = 0.0;

    if volume_5m_sol > 80.0 {
        score += 35.0;
    } else if volume_5m_sol > 40.0 {
        score += 25.0;
    } else if volume_5m_sol > 20.0 {
        score += 15.0;
    } else if volume_5m_sol > 5.0 {
        score += 5.0;
    }

    if buy_ratio > 0.78 {
        score += 30.0;
    } else if buy_ratio > 0.70 {
        score += 20.0;
    } else if buy_ratio > 0.62 {
        score += 10.0;
    }

    if volume_5m_sol > 0.0 && (volume_1m_sol > volume_5m_sol * 0.35) {
        score += 15.0;
    }

    if age_hours < 2.0 {
        score += 10.0;
    } else if age_hours < 12.0 {
        score += 5.0;
    }

    debug!(
        "Calculated score {:.2} (Vol 5m: {:.1} SOL, BuyRatio: {:.2}, Age: {:.1}h)",
        score, volume_5m_sol, buy_ratio, age_hours
    );

    score
}
" filePath="/workspace/app/src/scoring.rs