use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Metrics {
    articles_fetched: Arc<AtomicU64>,
    articles_failed: Arc<AtomicU64>,
    http_requests: Arc<AtomicU64>,
    http_failures: Arc<AtomicU64>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            articles_fetched: Arc::new(AtomicU64::new(0)),
            articles_failed: Arc::new(AtomicU64::new(0)),
            http_requests: Arc::new(AtomicU64::new(0)),
            http_failures: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_article_fetched(&self) {
        self.articles_fetched.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_article_failed(&self) {
        self.articles_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_http_request(&self) {
        self.http_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_http_failure(&self) {
        self.http_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub fn log_summary(&self) {
        info!(
            articles_fetched = self.articles_fetched.load(Ordering::Relaxed),
            articles_failed = self.articles_failed.load(Ordering::Relaxed),
            http_requests = self.http_requests.load(Ordering::Relaxed),
            http_failures = self.http_failures.load(Ordering::Relaxed),
            "Metrics summary"
        );
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
