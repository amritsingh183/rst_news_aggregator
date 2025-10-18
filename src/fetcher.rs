use crate::config::Config;
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
    config: Arc<Config>,
}

impl Fetcher {
    pub fn new(
        client: Client,
        cancel_token: CancellationToken,
        metrics: Metrics,
        config: &Config,
    ) -> Self {
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.requests_per_second));
        Self {
            client,
            rate_limiter,
            cancel_token,
            metrics,
            config: Arc::new(config.clone()),
        }
    }

    /// Fetch with retry logic and exponential backoff
    async fn fetch_with_retry<F, Fut>(&self, url: &str, operation: F) -> Result<String>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        if self.config.http.retry_attempts == 0 {
            return Err(AppError::ConfigError(
                "retry_attempts must be at least 1".into(),
            ));
        }

        let mut attempts = 0;
        let max_attempts = self.config.http.retry_attempts;

        loop {
            if self.cancel_token.is_cancelled() {
                return Err(AppError::ShutdownError);
            }

            self.rate_limiter.wait().await?;
            self.metrics.record_http_request();

            match operation().await {
                Ok(body) => return Ok(body),
                Err(e) => {
                    attempts += 1;
                    self.metrics.record_http_failure();

                    if attempts >= max_attempts {
                        return Err(AppError::http_error(
                            url,
                            format!("All {max_attempts} attempts failed: {e}"),
                        ));
                    }

                    let backoff =
                        self.config.retry_delay().as_millis() as u64 * 2u64.pow(attempts - 1);
                    warn!(
                        url,
                        attempt = attempts,
                        backoff_ms = backoff,
                        "Request failed, retrying"
                    );
                    sleep(std::time::Duration::from_millis(backoff)).await;
                }
            }
        }
    }

    pub async fn fetch_all(&self) -> Result<Vec<Article>> {
        let hn_fut = self.fetch_hacker_news();
        let rust_fut = self.fetch_rust_blog();

        let (hn_result, rust_result) = tokio::join!(hn_fut, rust_fut);

        let mut all_articles = Vec::new();

        match hn_result {
            Ok(mut articles) => {
                info!(count = articles.len(), "Fetched HackerNews articles");
                all_articles.append(&mut articles);
            }
            Err(e) => warn!(error = %e, "Failed to fetch HackerNews"),
        }

        match rust_result {
            Ok(mut articles) => {
                info!(count = articles.len(), "Fetched Rust Blog articles");
                all_articles.append(&mut articles);
            }
            Err(e) => warn!(error = %e, "Failed to fetch Rust Blog"),
        }

        if all_articles.is_empty() {
            return Err(AppError::NoArticlesError("all sources".into()));
        }

        Ok(all_articles)
    }

    async fn fetch_hacker_news(&self) -> Result<Vec<Article>> {
        let top_url = "https://hacker-news.firebaseio.com/v0/topstories.json";

        let body = self
            .fetch_with_retry(top_url, || {
                let client = self.client.clone();
                async move {
                    timeout(self.config.timeout(), client.get(top_url).send())
                        .await
                        .map_err(|_| AppError::TimeoutError(top_url.into()))?
                        .map_err(|e| AppError::http_error(top_url, e))?
                        .text()
                        .await
                        .map_err(|e| AppError::http_error(top_url, e))
                }
            })
            .await?;

        let ids: Vec<u64> = serde_json::from_str(&body)
            .map_err(|e| AppError::parse_error("HackerNews top stories", e))?;

        let ids_to_fetch: Vec<u64> = ids
            .into_iter()
            .take(self.config.fetcher.hacker_news_limit)
            .collect();

        let articles: Vec<Article> = stream::iter(ids_to_fetch)
            .map(|id| {
                let client = self.client.clone();
                let rate_limiter = self.rate_limiter.clone();
                let cancel_token = self.cancel_token.clone();
                let metrics = self.metrics.clone();
                let timeout_duration = self.config.timeout();
                let max_attempts = self.config.http.retry_attempts;
                let retry_delay = self.config.retry_delay();

                async move {
                    if cancel_token.is_cancelled() {
                        return Err(AppError::ShutdownError);
                    }

                    let url = format!("https://hacker-news.firebaseio.com/v0/item/{id}.json");

                    let mut attempts = 0;
                    loop {
                        rate_limiter.wait().await?;
                        metrics.record_http_request();

                        let result = timeout(timeout_duration, client.get(&url).send()).await;

                        match result {
                            Ok(Ok(response)) => match response.text().await {
                                Ok(text) => {
                                    let item: HackerNewsItem = serde_json::from_str(&text)
                                        .map_err(|e| AppError::parse_error("HackerNews item", e))?;

                                    let article_url = item.url.unwrap_or_else(|| {
                                        format!("https://news.ycombinator.com/item?id={}", item.id)
                                    });

                                    let mut article =
                                        Article::new(item.title, article_url, "HackerNews".into());
                                    if let Some(text) = item.text {
                                        article = article.with_description(text);
                                    }

                                    metrics.record_article_fetched();
                                    return Ok(article);
                                }
                                Err(e) => {
                                    attempts += 1;
                                    metrics.record_http_failure();

                                    if attempts >= max_attempts {
                                        metrics.record_article_failed();
                                        return Err(AppError::http_error(&url, e));
                                    }

                                    let backoff =
                                        retry_delay.as_millis() as u64 * 2u64.pow(attempts - 1);
                                    sleep(std::time::Duration::from_millis(backoff)).await;
                                }
                            },
                            Ok(Err(_)) | Err(_) => {
                                attempts += 1;
                                metrics.record_http_failure();

                                if attempts >= max_attempts {
                                    metrics.record_article_failed();
                                    return Err(AppError::http_error(
                                        &url,
                                        format!("Failed after {max_attempts} attempts"),
                                    ));
                                }

                                let backoff =
                                    retry_delay.as_millis() as u64 * 2u64.pow(attempts - 1);
                                sleep(std::time::Duration::from_millis(backoff)).await;
                            }
                        }
                    }
                }
            })
            .buffer_unordered(self.config.fetcher.max_concurrent_requests)
            .filter_map(|res| async {
                match res {
                    Ok(article) => Some(article),
                    Err(e) => {
                        warn!(error = %e, "Failed to fetch HN article");
                        None
                    }
                }
            })
            .collect()
            .await;

        Ok(articles)
    }

    async fn fetch_rust_blog(&self) -> Result<Vec<Article>> {
        let url = "https://blog.rust-lang.org/";

        let body = self
            .fetch_with_retry(url, || {
                let client = self.client.clone();
                async move {
                    timeout(self.config.timeout(), client.get(url).send())
                        .await
                        .map_err(|_| AppError::TimeoutError(url.into()))?
                        .map_err(|e| AppError::http_error(url, e))?
                        .text()
                        .await
                        .map_err(|e| AppError::http_error(url, e))
                }
            })
            .await?;

        let document = Html::parse_document(&body);

        // More robust selectors with validation
        let article_selector =
            Selector::parse("article.post, div.post, section.post").map_err(|e| {
                AppError::parse_error("Rust Blog", format!("Invalid article selector: {e}"))
            })?;

        let title_selector = Selector::parse("h2 a, h3 a, .post-title a").map_err(|e| {
            AppError::parse_error("Rust Blog", format!("Invalid title selector: {e}"))
        })?;

        let mut articles = Vec::new();

        for article_elem in document.select(&article_selector) {
            if let Some(link_elem) = article_elem.select(&title_selector).next() {
                let title = link_elem.text().collect::<String>().trim().to_string();

                if let Some(href) = link_elem.value().attr("href") {
                    // Validate URL is not empty
                    if href.is_empty() {
                        warn!("Skipping article with empty URL: {}", title);
                        continue;
                    }

                    let article_url = if href.starts_with("http") {
                        href.to_string()
                    } else {
                        format!("https://blog.rust-lang.org{}", href)
                    };

                    // Validate title is not empty
                    if title.is_empty() {
                        warn!("Skipping article with empty title at URL: {}", article_url);
                        continue;
                    }

                    let article = Article::new(title, article_url, "Rust Blog".into());
                    articles.push(article);
                    self.metrics.record_article_fetched();
                }
            }
        }

        if articles.is_empty() {
            return Err(AppError::NoArticlesError("Rust Blog".into()));
        }

        Ok(articles)
    }
}
