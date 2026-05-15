use prometheus::{IntCounter, IntCounterVec, Registry, TextEncoder};

pub struct Metrics {
    pub registry: Registry,
    pub ingest_accepted: IntCounter,
    pub ingest_signature_failed: IntCounter,
    pub ingest_unknown_route: IntCounter,
    pub ingest_bad_payload: IntCounter,
    pub delivery_attempts: IntCounterVec, // labels: route, outcome={success,failure}
    pub delivery_retries: IntCounter,
    pub delivery_dead: IntCounter,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();
        let ingest_accepted = IntCounter::new("sqlhook_ingest_accepted_total", "Accepted ingests").unwrap();
        let ingest_signature_failed = IntCounter::new("sqlhook_ingest_signature_failed_total", "Failed signature verifications").unwrap();
        let ingest_unknown_route = IntCounter::new("sqlhook_ingest_unknown_route_total", "Inbound to unknown route").unwrap();
        let ingest_bad_payload = IntCounter::new("sqlhook_ingest_bad_payload_total", "Inbound with invalid JSON").unwrap();
        let delivery_attempts = IntCounterVec::new(
            prometheus::Opts::new("sqlhook_delivery_attempts_total", "Delivery attempts"),
            &["route", "outcome"],
        ).unwrap();
        let delivery_retries = IntCounter::new("sqlhook_delivery_retries_total", "Deliveries scheduled for retry").unwrap();
        let delivery_dead = IntCounter::new("sqlhook_delivery_dead_total", "Deliveries moved to dead-letter").unwrap();

        registry.register(Box::new(ingest_accepted.clone())).unwrap();
        registry.register(Box::new(ingest_signature_failed.clone())).unwrap();
        registry.register(Box::new(ingest_unknown_route.clone())).unwrap();
        registry.register(Box::new(ingest_bad_payload.clone())).unwrap();
        registry.register(Box::new(delivery_attempts.clone())).unwrap();
        registry.register(Box::new(delivery_retries.clone())).unwrap();
        registry.register(Box::new(delivery_dead.clone())).unwrap();

        Self {
            registry,
            ingest_accepted,
            ingest_signature_failed,
            ingest_unknown_route,
            ingest_bad_payload,
            delivery_attempts,
            delivery_retries,
            delivery_dead,
        }
    }

    pub fn encode(&self) -> String {
        let encoder = TextEncoder::new();
        let mut buf = String::new();
        let metric_families = self.registry.gather();
        encoder.encode_utf8(&metric_families, &mut buf).unwrap();
        buf
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
