mod analyzer;
mod config;
mod error;
mod fetcher;
mod metrics;
mod model;
mod rate_limiter;

use crate::analyzer::ScoredArticle;
use crate::config::CONFIG;
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
    info!(
        timeout_secs = CONFIG.http.timeout_secs,
        max_concurrent = CONFIG.fetcher.max_concurrent_requests,
        rate_limit = CONFIG.rate_limit.requests_per_second,
        rayon_threads = CONFIG.analyzer.rayon_threads,
        "Configuration loaded"
    );

    analyzer::init_rayon_pool();

    let client = Client::builder()
        .timeout(CONFIG.http_timeout())
        .pool_max_idle_per_host(CONFIG.http.pool_max_idle_per_host)
        .user_agent("article-aggregator/0.1.0")
        .build()
        .map_err(|e| AppError::http_error("client", e))?;

    let cancel_token = CancellationToken::new();
    let shutdown_token = cancel_token.clone();

    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("Shutdown signal received, initiating graceful shutdown");
                shutdown_token.cancel();
            }
            Err(err) => {
                error!(error = %err, "Failed to listen for shutdown signal");
            }
        }
    });

    let metrics = Metrics::new();
    let fetcher = Fetcher::new(client, cancel_token.clone(), metrics.clone());

    info!("Fetching articles from all sources");

    let (hn_results, blog_results) =
        tokio::join!(fetcher.fetch_hacker_news(), fetcher.fetch_rust_blog());

    if cancel_token.is_cancelled() {
        info!("Shutdown requested, cleaning up");
        return Ok(());
    }

    let mut articles = Vec::new();

    for (source, results) in [("HackerNews", hn_results), ("Rust Blog", blog_results)] {
        for result in results {
            match result {
                Ok(article) => {
                    articles.push(article);
                    metrics.record_article_fetched();
                }
                Err(e) => {
                    warn!(source = source, error = %e, "Failed to fetch article");
                    metrics.record_article_failed();
                }
            }
        }
    }

    if articles.is_empty() {
        error!("No articles were successfully fetched");
        return Err(AppError::NoArticlesError("all sources".to_string()));
    }

    info!(
        article_count = articles.len(),
        "Successfully processed articles"
    );

    info!(
        keywords = ?CONFIG.keywords.values,
        "Analyzing articles with keywords"
    );

    let keywords = CONFIG.keywords.values.clone();

    //first ? handles JoinError, second handles ? Result<Vec<ScoredArticle>>
    let scored_articles =
        tokio::task::spawn_blocking(move || analyzer::score_articles(articles, &keywords))
            .await
            .map_err(|e| {
                AppError::parse_error("analyzer", format!("Analysis task panicked: {}", e))
            })??;

    if cancel_token.is_cancelled() {
        info!("Shutdown requested after analysis");
        return Ok(());
    }

    let mut ranked = scored_articles;
    ranked.sort_by(|a, b| {
        b.relevance_score()
            .partial_cmp(&a.relevance_score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    display_results(&ranked);
    metrics.log_summary();

    info!("Application completed successfully");
    Ok(())
}

fn display_results(ranked: &[ScoredArticle]) {
    info!("=== Top 10 Results ===");
    println!("\n{}", "=".repeat(80));
    println!("{:^80}", "TOP 10 ARTICLES");
    println!("{}", "=".repeat(80));

    for (i, item) in ranked.iter().take(10).enumerate() {
        println!(
            "\n{}. {} [Score: {:.2}]",
            i + 1,
            item.article().title(),
            item.relevance_score()
        );
        println!("   Source: {}", item.article().source());
        println!("   URL: {}", item.article().url());
        if !item.matched_keywords().is_empty() {
            println!("   Matched: {}", item.matched_keywords().join(", "));
        }
        println!("{}", "-".repeat(80));
    }
}
