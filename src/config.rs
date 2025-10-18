use crate::error::{AppError, Result};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::time::Duration;

use ::config::{Config as ConfigBuilder, Environment, File};

#[derive(Debug, Deserialize, Clone)]
pub struct HttpConfig {
    pub timeout_secs: u64,
    pub pool_max_idle_per_host: usize,
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FetcherConfig {
    pub max_concurrent_requests: usize,
    pub hacker_news_limit: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimitConfig {
    pub requests_per_second: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnalyzerConfig {
    pub rayon_threads: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct KeywordsConfig {
    pub values: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub http: HttpConfig,
    pub fetcher: FetcherConfig,
    pub rate_limit: RateLimitConfig,
    pub analyzer: AnalyzerConfig,
    pub keywords: KeywordsConfig,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config = ConfigBuilder::builder()
            .add_source(File::with_name("config").required(false))
            .add_source(Environment::with_prefix("APP"))
            .build()
            .map_err(|e| AppError::ConfigError(e.to_string()))?;

        let cfg: Self = config
            .try_deserialize()
            .map_err(|e| AppError::ConfigError(e.to_string()))?;

        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        if self.http.timeout_secs == 0 {
            return Err(AppError::ConfigError(
                "http.timeout_secs must be > 0".into(),
            ));
        }
        if self.fetcher.max_concurrent_requests == 0 {
            return Err(AppError::ConfigError(
                "fetcher.max_concurrent_requests must be > 0".into(),
            ));
        }
        if self.rate_limit.requests_per_second == 0 {
            return Err(AppError::ConfigError(
                "rate_limit.requests_per_second must be > 0".into(),
            ));
        }
        if self.analyzer.rayon_threads == 0 {
            return Err(AppError::ConfigError(
                "analyzer.rayon_threads must be > 0".into(),
            ));
        }
        Ok(())
    }

    pub fn http_timeout(&self) -> Duration {
        Duration::from_secs(self.http.timeout_secs)
    }

    pub fn retry_delay(&self) -> Duration {
        Duration::from_millis(self.http.retry_delay_ms)
    }
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    Config::load().unwrap_or_else(|_| {
        eprintln!("Failed to load config, using defaults");
        Config::default()
    })
});

impl Default for Config {
    fn default() -> Self {
        Self {
            http: HttpConfig {
                timeout_secs: 10,
                pool_max_idle_per_host: 10,
                retry_attempts: 3,
                retry_delay_ms: 1000,
            },
            fetcher: FetcherConfig {
                max_concurrent_requests: 10,
                hacker_news_limit: 15,
            },
            rate_limit: RateLimitConfig {
                requests_per_second: 5,
            },
            analyzer: AnalyzerConfig {
                rayon_threads: num_cpus::get(), // dynamic
            },
            keywords: KeywordsConfig {
                values: vec![
                    "rust".to_string(),
                    "ai".to_string(),
                    "performance".to_string(),
                    "async".to_string(),
                ],
            },
        }
    }
}
