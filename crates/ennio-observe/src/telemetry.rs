use opentelemetry_otlp::SpanExporter;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;

pub struct TelemetryGuard {
    _provider: SdkTracerProvider,
}

pub fn init_otlp(
    service_name: &str,
    endpoint: &str,
) -> Result<TelemetryGuard, Box<dyn std::error::Error + Send + Sync>> {
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_service_name(service_name.to_string())
                .build(),
        )
        .build();

    Ok(TelemetryGuard {
        _provider: provider,
    })
}
