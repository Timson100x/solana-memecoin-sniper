use tracing::debug;

pub fn calculate_score(
    volume_5m_sol: f64,
    volume_1m_sol: f64,
    buy_ratio: f64,
    age_hours: f64,
) -> f64 {
    let mut score: f64 = 0.0;

    if volume_5m_sol > 120.0 {
        score += 35.0;
    } else if volume_5m_sol > 80.0 {
        score += 30.0;
    } else if volume_5m_sol > 50.0 {
        score += 25.0;
    } else if volume_5m_sol > 30.0 {
        score += 18.0;
    } else if volume_5m_sol > 15.0 {
        score += 12.0;
    } else if volume_5m_sol > 5.0 {
        score += 5.0;
    }

    if buy_ratio > 0.82 {
        score += 30.0;
    } else if buy_ratio > 0.78 {
        score += 26.0;
    } else if buy_ratio > 0.72 {
        score += 20.0;
    } else if buy_ratio > 0.65 {
        score += 12.0;
    } else if buy_ratio > 0.55 {
        score += 6.0;
    }

    if volume_5m_sol > 1.0 {
        let accel = volume_1m_sol / volume_5m_sol;
        if accel >= 0.45 {
            score += 15.0;
        } else if accel >= 0.35 {
            score += 12.0;
        } else if accel >= 0.25 {
            score += 8.0;
        } else if accel >= 0.18 {
            score += 4.0;
        }
    }

    if age_hours < 1.0 {
        score += 10.0;
    } else if age_hours < 3.0 {
        score += 7.0;
    } else if age_hours < 8.0 {
        score += 4.0;
    } else if age_hours < 24.0 {
        score += 2.0;
    }

    debug!(
        "score={:.1} | vol5m={:.1} | vol1m={:.1} | buy={:.2} | age={:.1}h",
        score, volume_5m_sol, volume_1m_sol, buy_ratio, age_hours
    );

    score.clamp(0.0_f64, 100.0_f64)
}
