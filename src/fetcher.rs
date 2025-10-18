use crate::config::CONFIG;
use crate::error::{AppError, Result};
use crate::metrics::Metrics;
use crate::model::{Article, HackerNewsItem};
use crate::rate_limiter::RateLimiter;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use scraper::{Html, Selector};
use std::sync::Arc;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

pub struct Fetcher {
    client: Client,
    rate_limiter: Arc<RateLimiter>,
    cancel_token: CancellationToken,
    metrics: Metrics,
}

impl Fetcher {
    pub fn new(client: Client, cancel_token: CancellationToken, metrics: Metrics) -> Self {
        let rate_limiter = Arc::new(RateLimiter::new(CONFIG.rate_limit.requests_per_second));

        Self {
            client,
            rate_limiter,
            cancel_token,
            metrics,
        }
    }

    /// Fetch with retry logic and exponential backoff
    async fn fetch_with_retry<F, Fut, T>(&self, url: &str, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempts = 0;
        let max_attempts = CONFIG.http.retry_attempts;

        loop {
            if self.cancel_token.is_cancelled() {
                return Err(AppError::ShutdownError);
            }

            self.rate_limiter.wait().await?;
            self.metrics.record_http_request();

            attempts += 1;
            match timeout(CONFIG.http_timeout(), operation()).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(e)) if attempts >= max_attempts => {
                    self.metrics.record_http_failure();
                    warn!(
                        url = url,
                        attempts = attempts,
                        error = %e,
                        "Max retry attempts reached"
                    );
                    return Err(e);
                }
                Ok(Err(e)) => {
                    self.metrics.record_http_failure();
                    let delay = CONFIG.retry_delay() * 2_u32.pow(attempts - 1);
                    warn!(
                        url = url,
                        attempt = attempts,
                        error = %e,
                        retry_delay_ms = delay.as_millis(),
                        "Request failed, retrying"
                    );
                    sleep(delay).await;
                }
                Err(_) => {
                    if attempts >= max_attempts {
                        return Err(AppError::TimeoutError(format!(
                            "Request to {} timed out after {} attempts",
                            url, attempts
                        )));
                    }
                    warn!(url = url, attempt = attempts, "Request timed out, retrying");
                    sleep(CONFIG.retry_delay()).await;
                }
            }
        }
    }

    /// Fetch articles from Hacker News API
    pub async fn fetch_hacker_news(&self) -> Vec<Result<Article>> {
        let url = "https://hacker-news.firebaseio.com/v0/topstories.json";
        info!("Fetching Hacker News top stories");

        // Step 1: Fetch story IDs
        let story_ids: Vec<u64> = match self
            .fetch_with_retry(url, || async {
                let response = self
                    .client
                    .get(url)
                    .send()
                    .await
                    .map_err(|e| AppError::http_error(url, e))?;

                response
                    .error_for_status()
                    .map_err(|e| AppError::http_error(url, e))?
                    .json()
                    .await
                    .map_err(|e| AppError::parse_error("HackerNews", e))
            })
            .await
        {
            Ok(ids) => ids,
            Err(e) => return vec![Err(e)],
        };

        info!(story_count = story_ids.len(), "Fetched story IDs");

        // Step 2: Fetch individual stories concurrently
        let futures = story_ids
            .into_iter()
            .take(CONFIG.fetcher.hacker_news_limit)
            .map(|id| {
                let client = self.client.clone();
                let rate_limiter = self.rate_limiter.clone();
                let cancel_token = self.cancel_token.clone();
                let metrics = self.metrics.clone();

                async move {
                    if cancel_token.is_cancelled() {
                        return Err(AppError::ShutdownError);
                    }

                    rate_limiter.wait().await?;
                    metrics.record_http_request();

                    let url = format!("https://hacker-news.firebaseio.com/v0/item/{}.json", id);
                    let response = client
                        .get(&url)
                        .send()
                        .await
                        .map_err(|e| AppError::http_error(&url, e))?;

                    let item: HackerNewsItem = response
                        .error_for_status()
                        .map_err(|e| AppError::http_error(&url, e))?
                        .json()
                        .await
                        .map_err(|e| AppError::parse_error("HackerNews", e))?;

                    let article = Article::new(
                        item.title,
                        item.url.unwrap_or_else(|| {
                            format!("https://news.ycombinator.com/item?id={}", item.id)
                        }),
                        "HackerNews".to_string(),
                    );

                    Ok(if let Some(text) = item.text {
                        article.with_description(text)
                    } else {
                        article
                    })
                }
            });

        stream::iter(futures)
            .buffer_unordered(CONFIG.fetcher.max_concurrent_requests)
            .collect::<Vec<_>>()
            .await
    }

    /// Fetch articles from Rust Blog
    pub async fn fetch_rust_blog(&self) -> Vec<Result<Article>> {
        const BLOG_URL: &str = "https://blog.rust-lang.org/";
        info!("Fetching Rust Blog articles");

        if self.cancel_token.is_cancelled() {
            return vec![Err(AppError::ShutdownError)];
        }

        // Fetch HTML
        let html_content = match self
            .fetch_with_retry(BLOG_URL, || async {
                let response = self
                    .client
                    .get(BLOG_URL)
                    .send()
                    .await
                    .map_err(|e| AppError::http_error(BLOG_URL, e))?;

                response
                    .error_for_status()
                    .map_err(|e| AppError::http_error(BLOG_URL, e))?
                    .text()
                    .await
                    .map_err(|e| AppError::parse_error("Rust Blog", e))
            })
            .await
        {
            Ok(html) => html,
            Err(e) => return vec![Err(e)],
        };

        // Parse HTML in blocking thread
        match tokio::task::spawn_blocking(move || -> Result<Vec<Article>> {
            let document = Html::parse_document(&html_content);

            let row_selector = Selector::parse("table tr")
                .map_err(|e| AppError::parse_error("Rust Blog", e.to_string()))?;
            let link_selector = Selector::parse("a")
                .map_err(|e| AppError::parse_error("Rust Blog", e.to_string()))?;

            let mut articles = Vec::new();

            for row in document.select(&row_selector) {
                if let Some(link) = row.select(&link_selector).next() {
                    let title = link.text().collect::<String>().trim().to_string();

                    if title.is_empty() || title.starts_with("Posts in") {
                        continue;
                    }

                    if let Some(href) = link.value().attr("href") {
                        let url = if href.starts_with("http") {
                            href.to_string()
                        } else {
                            format!("{}{}", BLOG_URL.trim_end_matches('/'), href)
                        };

                        articles.push(Article::new(title, url, "Rust Blog".to_string()));
                    }
                }
            }

            if articles.is_empty() {
                return Err(AppError::NoArticlesError("Rust Blog".to_string()));
            }

            Ok(articles)
        })
        .await
        {
            Err(e) => vec![Err(AppError::parse_error(
                "Rust Blog",
                format!("Task panicked: {}", e),
            ))],
            Ok(result) => match result {
                Ok(articles) => {
                    info!(article_count = articles.len(), "Fetched Rust Blog articles");
                    articles.into_iter().map(Ok).collect()
                }
                Err(e) => vec![Err(e)],
            },
        }
    }
}
