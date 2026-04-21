mod app;
mod config;
mod error;
mod routes;
mod state;
mod store;

use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

use crate::store::Store;

mod telemetry {
    use std::time::Duration;

    use anyhow::{Context, Result};
    use opentelemetry::global;
    use opentelemetry::propagation::TextMapCompositePropagator;
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};
    use opentelemetry_sdk::trace::{
        BatchConfigBuilder, BatchSpanProcessor, Sampler, SdkTracerProvider,
    };
    use opentelemetry_sdk::Resource;
    use tracing::Level;
    use tracing_appender::non_blocking::WorkerGuard;
    use tracing_subscriber::filter::Targets;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Layer;

    use crate::config::TelemetryConfig;

    pub struct TelemetryHandle {
        tracer_provider: Option<SdkTracerProvider>,
        _log_guard: WorkerGuard,
    }

    impl TelemetryHandle {
        pub fn shutdown(self) {
            if let Some(tracer_provider) = self.tracer_provider {
                let _ = tracer_provider.shutdown();
            }
        }
    }

    pub fn init(config: &TelemetryConfig) -> Result<TelemetryHandle> {
        global::set_text_map_propagator(TextMapCompositePropagator::new(vec![
            Box::new(TraceContextPropagator::new()),
            Box::new(BaggagePropagator::new()),
        ]));

        let (log_writer, log_guard) = tracing_appender::non_blocking(std::io::stdout());
        let fmt_layer = tracing_subscriber::fmt::layer()
            .json()
            .flatten_event(true)
            .with_current_span(true)
            .with_writer(log_writer)
            .with_target(false)
            .with_ansi(false)
            .without_time();
        let targets = Targets::new()
            .with_target("fishystuff_server", Level::TRACE)
            .with_default(Level::WARN);

        let tracer_provider = if config.enabled && !config.otlp_traces_endpoint.is_empty() {
            Some(build_tracer_provider(config)?)
        } else {
            None
        };

        let otel_layer = tracer_provider.as_ref().map(|provider| {
            tracing_opentelemetry::layer()
                .with_tracer(provider.tracer(config.service_name.clone()))
                .with_filter(targets.clone())
        });

        tracing_subscriber::registry()
            .with(fmt_layer.with_filter(targets))
            .with(otel_layer)
            .try_init()
            .context("initialize tracing subscriber")?;

        Ok(TelemetryHandle {
            tracer_provider,
            _log_guard: log_guard,
        })
    }

    fn build_tracer_provider(config: &TelemetryConfig) -> Result<SdkTracerProvider> {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(config.otlp_traces_endpoint.clone())
            .with_timeout(Duration::from_secs(4))
            .build()
            .context("build OTLP trace exporter")?;

        let span_processor = BatchSpanProcessor::builder(exporter)
            .with_batch_config(
                BatchConfigBuilder::default()
                    .with_max_queue_size(512)
                    .with_max_export_batch_size(32)
                    .with_scheduled_delay(Duration::from_millis(250))
                    .build(),
            )
            .build();

        let resource = Resource::builder_empty()
            .with_attributes([
                KeyValue::new("service.name", config.service_name.clone()),
                KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                KeyValue::new(
                    "deployment.environment",
                    config.deployment_environment.clone(),
                ),
            ])
            .build();

        Ok(SdkTracerProvider::builder()
            .with_resource(resource)
            .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                config.sample_ratio,
            ))))
            .with_span_processor(span_processor)
            .build())
    }
}

fn spawn_startup_cache_prewarm(store: Arc<dyn Store>) {
    tokio::spawn(async move {
        let mut last_err = None;
        for attempt in 0..5 {
            match store.prime_startup_caches().await {
                Ok(()) => {
                    last_err = None;
                    break;
                }
                Err(err) => {
                    last_err = Some(err);
                    if attempt < 4 {
                        tokio::time::sleep(Duration::from_millis(250 * (attempt + 1) as u64)).await;
                    }
                }
            }
        }
        if let Some(err) = last_err {
            warn!(error = ?err, "startup cache prewarm failed after retries");
        }
    });
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::AppConfig::parse()?;
    let telemetry = telemetry::init(&config.telemetry)?;
    let bind = config.bind.clone();
    let state = state::AppState::new(config)?;
    spawn_startup_cache_prewarm(state.store.clone());
    let app = app::build_router(state);

    let addr = bind
        .parse()
        .with_context(|| format!("invalid bind address {bind}"))?;
    info!(%addr, "fishystuff_server listening");

    let serve_result = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .context("serve axum");

    telemetry.shutdown();

    serve_result
}
