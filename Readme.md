# New to Rust? 

Checkout my articles at https://amritsingh183.github.io/

# News aggregator in Rust 

A fast, async Rust tool that fetches top stories from Hacker News, enriches them into typed articles, scores relevance using Aho–Corasick keyword matching in parallel with Rayon, and exposes clean building blocks for serialization, metrics, and robust error handling.

## Why this project
- Strong focus on memory safety and performance via Rust’s ownership model, zero-cost abstractions, and data-parallel scoring with Rayon for multi-core efficiency.
- Reliable async networking with Tokio and Reqwest, rate-limited concurrency, and resilient retry/timeout logic for production scraping pipelines.

## Features
- Async fetch of Hacker News top story IDs and items using Reqwest on Tokio runtime with structured retries and timeouts.
- CPU-bound relevance scoring in parallel using Rayon and Aho–Corasick over normalized article text, returning matched keywords and a numeric score per article.
- Centralized configuration via file and environment variables with once_cell Lazy initialization and serde-based deserialization.
- Unified error type with thiserror and ergonomic propagation using Result<T> and the ? operator across async and threaded boundaries.
- Lightweight metrics via Arc<AtomicU64> counters for requests, successes, and failures without locking overhead.
- Clean data model with serde Serialize/Deserialize for easy JSON/TOML interop and encapsulated getters for API clarity.

## Quick start
- Install Rust and Cargo, then clone the repository and build with cargo to get started quickly on any platform supported by Rust.
- Run the binary in debug or release mode, and customize behavior through a config file or environment variables described below.

## Installation
- Ensure Rust toolchain is installed, then use Cargo to build and run the project in-place for development or package it for deployment with release builds.
- Recommended: use release builds for best performance in high-concurrency fetch and parallel scoring phases.

## Run
- Execute the program with Cargo to start asynchronous fetching and subsequent parallel analysis of articles.
- The runtime uses a multi-threaded Tokio flavor and offloads CPU-bound scoring to a Rayon pool via spawn_blocking for optimal throughput.

```
# Build and run
cargo run

# Faster execution
cargo run --release
```

## Configuration
- The application loads configuration from an optional config.toml and environment variables with an APP_ prefix using the config crate integration.
- A thread-safe Lazy global holds the parsed Config, falling back to sensible defaults on failure to deserialize or missing files.

Sample config.toml:
```
[http]
timeout_secs = 15
pool_max_idle_per_host = 20
retry_attempts = 5
retry_delay_ms = 2000

[fetcher]
max_concurrent_requests = 20
hacker_news_limit = 30

[rate_limit]
requests_per_second = 10

[analyzer]
rayon_threads = 8

[keywords]
values = ["rust", "async", "tokio", "performance"]
```

Environment overrides (examples):
```
export APP_HTTP_TIMEOUT_SECS=20
export APP_FETCHER_MAX_CONCURRENT_REQUESTS=50
export APP_KEYWORDS_VALUES__0=rust
export APP_KEYWORDS_VALUES__1=async
export APP_KEYWORDS_VALUES__2=web
cargo run
```

## How it works
- The fetcher retrieves top story IDs and then item details concurrently with bounded in-flight futures, retry-on-failure, and per-request timeouts for robustness under transient network issues.
- Articles are converted into a domain struct with title, url, source, and optional description, then scored in parallel by building a shared Aho–Corasick automaton over configured keywords.
- Each article’s searchable_text is lowercased and scanned, producing a relevance_score and matched_keywords, then collected into a vector of ScoredArticle for downstream use or serialization.

## Data model
- Article: title, url, source, description: Option<String>, with computed searchable_text that concatenates title and description when present for better matching coverage.
- ScoredArticle: article, relevance_score: f64, matched_keywords: Vec<String>, with read-only getters to keep fields private and API surface minimal.

## Error handling
- A single AppError enum captures HTTP, parse, timeout, rate limit, config, analyzer, and shutdown errors with rich Display formatting via thiserror.
- All fallible operations return Result<T>, with ? for early returns and map_err for precise error context conversion from library errors into AppError.

## Concurrency and parallelism
- Async I/O uses Tokio for non-blocking HTTP, stream buffering with buffer_unordered, and join! for concurrent tasks to maximize network throughput.
- CPU-bound scoring uses Rayon’s parallel iterators, sharing the compiled Aho–Corasick automaton across threads with Arc for minimal cloning overhead.

## Rate limiting and metrics
- An internal rate limiter built atop governor constrains request rate per second to avoid remote throttling and to smooth bursty fetch patterns.
- Atomic counters tally request attempts, successes, and failures to quantify health and performance without introducing locks or contention.

## Project structure
- src/main.rs: async entrypoint (multi-thread runtime), orchestration, and bridging async fetch with blocking parallel analysis safely.
- src/fetcher.rs: networking, retries, timeouts, concurrency control, and HN item mapping into Article instances.
- src/analyzer.rs: keyword automaton build, per-article scoring in parallel, and aggregation into ScoredArticle outputs.
- src/model.rs: Article and related types with serde traits and encapsulated getters plus computed fields.
- src/error.rs: AppError and Result<T> alias to unify error flows with thiserror.
- src/config.rs: Config schema, defaults, validation, and Lazy global initialization.
- src/metrics.rs: Arc<AtomicU64>-based counters and helpers for lightweight instrumentation.
- src/rate_limiter.rs: governor-backed limiter type aliases and helpers for request pacing.

Example tree (abridged):
```
src/
  analyzer.rs
  config.rs
  error.rs
  fetcher.rs
  metrics.rs
  model.rs
  rate_limiter.rs
  main.rs
```

## Dependencies (core crates)
- tokio: async runtime for non-blocking I/O and task scheduling.
- reqwest: HTTP client with async support and JSON helpers.
- rayon: data-parallel iterators and work-stealing thread pool.
- aho-corasick: fast multi-pattern search for keyword matching.
- serde + serde_json: serialization/deserialization for config and data.
- thiserror: ergonomic error definitions and Display impls.
- once_cell: thread-safe Lazy initialization of global Config.
- governor: rate limiting to bound outbound request rate.
- futures: stream utilities like buffer_unordered for concurrent pipelines.
- num_cpus: determine optimal Rayon thread count defaults per host.

## Sample output shape
- Articles and scored results are serializable via serde and can be logged or exported as JSON depending on integration in main.rs.
- Example JSON documents for downstream processing can include matched keyword lists and normalized scores alongside original article metadata.

```
{
  "article": {
    "title": "Rust Async",
    "url": "https://example.com",
    "source": "HN",
    "description": "A great article"
  },
  "relevance_score": 0.87,
  "matched_keywords": ["rust", "async"]
}
```

## Development workflow
- Use cargo fmt and cargo clippy to maintain quality and consistency across modules and ensure zero-cost abstractions remain well-optimized.
- Split I/O-bound and CPU-bound work cleanly, and prefer Arc over Rc for any cross-thread sharing to satisfy Send/Sync and avoid data races.

## Design notes
- Encapsulation with private fields and public getters maintains invariants while providing a small, stable API surface for integration.
- Trait-driven, generic utilities like fetch_with_retry<F, Fut, T> keep async workflows composable and testable without runtime overhead.

## Disclaimer
- Ensure scraping adheres to target site policies and apply conservative rate limits to remain a good citizen of the web.
