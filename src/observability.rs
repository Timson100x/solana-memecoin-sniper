use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::{fmt, EnvFilter};

/// Initialise structured tracing logging. No sensitive values are ever logged; callers
/// are responsible for masking secrets before emitting log lines.
pub fn init_logging() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let timer = UtcTime::rfc_3339();

    fmt()
        .with_env_filter(filter)
        .with_timer(timer)
        .with_thread_names(true)
        .with_target(false)
        .compact()
        .init();
}
