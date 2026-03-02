use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn default_env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

pub fn init_logging() {
    tracing_subscriber::registry()
        .with(default_env_filter())
        .with(tracing_subscriber::fmt::layer())
        .init();
}

pub fn init_json_logging() {
    tracing_subscriber::registry()
        .with(default_env_filter())
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}
