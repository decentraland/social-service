use std::collections::HashMap;

use actix_web_prom::{PrometheusMetrics, PrometheusMetricsBuilder};

pub fn initialize_metrics(environment: String) -> PrometheusMetrics {
    let mut map = HashMap::new();
    map.insert(String::from("env"), environment);
    PrometheusMetricsBuilder::new("api")
        .endpoint("/metrics")
        .const_labels(map)
        .build()
        .unwrap()
}
