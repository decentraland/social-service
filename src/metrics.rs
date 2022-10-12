use std::collections::HashMap;

use actix_web_prom::{PrometheusMetrics, PrometheusMetricsBuilder};

pub fn initializeMetrics() -> PrometheusMetrics {
    let mut labels = HashMap::new();
    labels.insert("label1".to_string(), "value1".to_string());

    PrometheusMetricsBuilder::new("api")
        .endpoint("/metrics")
        .const_labels(labels)
        .build()
        .unwrap()
}
