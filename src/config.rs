use crate::error::{AppError, Result};
use config::{Config as ConfigBuilder, Environment, File};
use serde::Deserialize;
use std::time::Duration;

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
	/// Load configuration from config.toml and environment variables
	/// Returns an error if configuration is invalid rather than using defaults
	pub fn load() -> Result<Self> {
		let settings = ConfigBuilder::builder()
			.add_source(File::with_name("config").required(false))
			.add_source(Environment::with_prefix("APP").separator("__"))
			.build()
			.map_err(|e| AppError::ConfigError(format!("Failed to build config: {e}")))?;

		let config: Config = settings
			.try_deserialize()
			.map_err(|e| AppError::ConfigError(format!("Failed to deserialize config: {e}")))?;

		// Validate configuration
		config.validate()?;

		Ok(config)
	}

	/// Validate configuration values
	fn validate(&self) -> Result<()> {
		if self.http.timeout_secs == 0 {
			return Err(AppError::ConfigError("timeout_secs must be greater than 0".into()));
		}
		if self.http.retry_attempts == 0 {
			return Err(AppError::ConfigError("retry_attempts must be greater than 0".into()));
		}
		if self.rate_limit.requests_per_second == 0 {
			return Err(AppError::ConfigError(
				"requests_per_second must be greater than 0".into(),
			));
		}
		if self.fetcher.max_concurrent_requests == 0 {
			return Err(AppError::ConfigError(
				"max_concurrent_requests must be greater than 0".into(),
			));
		}
		if self.analyzer.rayon_threads == 0 {
			return Err(AppError::ConfigError("rayon_threads must be greater than 0".into()));
		}
		if self.keywords.values.is_empty() {
			return Err(AppError::ConfigError("keywords list cannot be empty".into()));
		}
		Ok(())
	}

	pub const fn timeout(&self) -> Duration {
		Duration::from_secs(self.http.timeout_secs)
	}

	pub fn retry_delay(&self) -> Duration {
		Duration::from_millis(self.http.retry_delay_ms)
	}
}
