use actix_web_prom::{PrometheusMetrics, PrometheusMetricsBuilder};

pub fn initialize_metrics() -> PrometheusMetrics {
    PrometheusMetricsBuilder::new("api")
        .endpoint("/metrics")
        .build()
        .unwrap()
}
