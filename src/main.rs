mod analyzer;
mod config;
mod error;
mod fetcher;
mod metrics;
mod model;
mod rate_limiter;

use crate::analyzer::ScoredArticle;
use crate::config::Config;
use crate::error::{AppError, Result};
use crate::fetcher::Fetcher;
use crate::metrics::Metrics;
use reqwest::Client;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .with_thread_ids(true)
        .json()
        .init();

    info!("Starting article aggregator");

    // Load and validate configuration explicitly
    let config = Config::load()?;
    
    info!(
        timeout_secs = config.http.timeout_secs,
        max_concurrent = config.fetcher.max_concurrent_requests,
        rate_limit = config.rate_limit.requests_per_second,
        rayon_threads = config.analyzer.rayon_threads,
        "Configuration loaded"
    );

    analyzer::init_rayon_pool(config.analyzer.rayon_threads);

    let client = Client::builder()
        .timeout(config.timeout())
        .pool_max_idle_per_host(config.http.pool_max_idle_per_host)
        .build()
        .map_err(|e| AppError::ConfigError(format!("Failed to build HTTP client: {e}")))?;

    let cancel_token = CancellationToken::new();
    let cancel_clone = cancel_token.clone();

    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("Shutdown signal received");
                cancel_clone.cancel();
            }
            Err(err) => {
                error!(error = %err, "Failed to listen for shutdown signal");
            }
        }
    });

    let metrics = Metrics::new();
    let fetcher = Fetcher::new(client, cancel_token.clone(), metrics.clone(), &config);

    match run_aggregator(fetcher, &config).await {
        Ok(scored) => {
            display_results(&scored);
            metrics.log_summary();
            Ok(())
        }
        Err(e) if matches!(e, AppError::ShutdownError) => {
            warn!("Gracefully shutting down");
            metrics.log_summary();
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Aggregator failed");
            metrics.log_summary();
            Err(e)
        }
    }
}

async fn run_aggregator(fetcher: Fetcher, config: &Config) -> Result<Vec<ScoredArticle>> {
    let articles = fetcher.fetch_all().await?;

    if articles.is_empty() {
        warn!("No articles fetched from any source");
        return Ok(Vec::new());
    }

    info!(count = articles.len(), "Fetched articles successfully");

    let mut scored = analyzer::score_articles(articles, &config.keywords.values)?;
    
    // Filter out NaN scores and sort
    scored.retain(|article| article.relevance_score().is_finite());
    scored.sort_by(|a, b| {
        b.relevance_score()
            .partial_cmp(&a.relevance_score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(scored)
}

fn display_results(articles: &[ScoredArticle]) {
    info!("=== Top Relevant Articles ===");
    for (i, scored) in articles.iter().take(10).enumerate() {
        info!(
            rank = i + 1,
            score = format!("{:.2}", scored.relevance_score()),
            title = scored.article().title(),
            source = scored.article().source(),
            url = scored.article().url(),
            keywords = ?scored.matched_keywords(),
        );
    }
}
