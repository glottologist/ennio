use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_exporter_prometheus::PrometheusHandle;

pub fn init_prometheus() -> Result<PrometheusHandle, metrics_exporter_prometheus::BuildError> {
    PrometheusBuilder::new().install_recorder()
}

pub fn register_metrics() {
    metrics::describe_counter!(
        "ennio_sessions_spawned_total",
        "Total number of sessions spawned"
    );
    metrics::describe_counter!(
        "ennio_sessions_killed_total",
        "Total number of sessions killed"
    );
    metrics::describe_counter!(
        "ennio_sessions_completed_total",
        "Total number of sessions completed"
    );
    metrics::describe_gauge!(
        "ennio_sessions_active",
        "Number of currently active sessions"
    );
    metrics::describe_histogram!(
        "ennio_session_duration_seconds",
        "Duration of sessions in seconds"
    );
    metrics::describe_counter!("ennio_events_total", "Total number of events emitted");
    metrics::describe_counter!(
        "ennio_plugin_calls_total",
        "Total plugin calls by slot and plugin name"
    );
    metrics::describe_histogram!(
        "ennio_plugin_call_duration_seconds",
        "Plugin call duration in seconds"
    );
    metrics::describe_counter!("ennio_cost_usd_total", "Total cost in USD");
    metrics::describe_counter!(
        "ennio_reactions_triggered_total",
        "Total reactions triggered"
    );
    metrics::describe_counter!(
        "ennio_reactions_escalated_total",
        "Total reactions escalated to human"
    );
}
