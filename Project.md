# rst_news_aggregator[1]

A fast, async Rust tool that fetches top stories from Hacker News, enriches them into typed articles, scores relevance using Aho–Corasick keyword matching in parallel with Rayon, and exposes clean building blocks for serialization, metrics, and robust error handling.[1]

### Why this project[1]
- Strong focus on memory safety and performance via Rust’s ownership model, zero-cost abstractions, and data-parallel scoring with Rayon for multi-core efficiency.[1]
- Reliable async networking with Tokio and Reqwest, rate-limited concurrency, and resilient retry/timeout logic for production scraping pipelines.[1]

### Features[1]
- Async fetch of Hacker News top story IDs and items using Reqwest on Tokio runtime with structured retries and timeouts.[1]
- CPU-bound relevance scoring in parallel using Rayon and Aho–Corasick over normalized article text, returning matched keywords and a numeric score per article.[1]
- Centralized configuration via file and environment variables with once_cell Lazy initialization and serde-based deserialization.[1]
- Unified error type with thiserror and ergonomic propagation using Result<T> and the ? operator across async and threaded boundaries.[1]
- Lightweight metrics via Arc<AtomicU64> counters for requests, successes, and failures without locking overhead.[1]
- Clean data model with serde Serialize/Deserialize for easy JSON/TOML interop and encapsulated getters for API clarity.[1]

### Quick start[1]
- Install Rust and Cargo, then clone the repository and build with cargo to get started quickly on any platform supported by Rust.[1]
- Run the binary in debug or release mode, and customize behavior through a config file or environment variables described below.[1]

### Installation[1]
- Ensure Rust toolchain is installed, then use Cargo to build and run the project in-place for development or package it for deployment with release builds.[1]
- Recommended: use release builds for best performance in high-concurrency fetch and parallel scoring phases.[1]

### Run[1]
- Execute the program with Cargo to start asynchronous fetching and subsequent parallel analysis of articles.[1]
- The runtime uses a multi-threaded Tokio flavor and offloads CPU-bound scoring to a Rayon pool via spawn_blocking for optimal throughput.[1]

```bash
# Build and run
cargo run [attached_file:1]

# Faster execution
cargo run --release [attached_file:1]
```

### Configuration[1]
- The application loads configuration from an optional config.toml and environment variables with an APP_ prefix using the config crate integration.[1]
- A thread-safe Lazy global holds the parsed Config, falling back to sensible defaults on failure to deserialize or missing files.[1]

Sample config.toml :[1]
```toml
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

Environment overrides (examples) :[1]
```bash
export APP_HTTP_TIMEOUT_SECS=20
export APP_FETCHER_MAX_CONCURRENT_REQUESTS=50
export APP_KEYWORDS_VALUES__0=rust
export APP_KEYWORDS_VALUES__1=async
export APP_KEYWORDS_VALUES__2=web
cargo run
```

### How it works[1]
- The fetcher retrieves top story IDs and then item details concurrently with bounded in-flight futures, retry-on-failure, and per-request timeouts for robustness under transient network issues.[1]
- Articles are converted into a domain struct with title, url, source, and optional description, then scored in parallel by building a shared Aho–Corasick automaton over configured keywords.[1]
- Each article’s searchable_text is lowercased and scanned, producing a relevance_score and matched_keywords, then collected into a vector of ScoredArticle for downstream use or serialization.[1]

### Data model[1]
- Article: title, url, source, description: Option<String>, with computed searchable_text that concatenates title and description when present for better matching coverage.[1]
- ScoredArticle: article, relevance_score: f64, matched_keywords: Vec<String>, with read-only getters to keep fields private and API surface minimal.[1]

### Error handling[1]
- A single AppError enum captures HTTP, parse, timeout, rate limit, config, analyzer, and shutdown errors with rich Display formatting via thiserror.[1]
- All fallible operations return Result<T>, with ? for early returns and map_err for precise error context conversion from library errors into AppError.[1]

### Concurrency and parallelism[1]
- Async I/O uses Tokio for non-blocking HTTP, stream buffering with buffer_unordered, and join! for concurrent tasks to maximize network throughput.[1]
- CPU-bound scoring uses Rayon’s parallel iterators, sharing the compiled Aho–Corasick automaton across threads with Arc for minimal cloning overhead.[1]

### Rate limiting and metrics[1]
- An internal rate limiter built atop governor constrains request rate per second to avoid remote throttling and to smooth bursty fetch patterns.[1]
- Atomic counters tally request attempts, successes, and failures to quantify health and performance without introducing locks or contention.[1]

### Project structure[1]
- src/main.rs: async entrypoint (multi-thread runtime), orchestration, and bridging async fetch with blocking parallel analysis safely.[1]
- src/fetcher.rs: networking, retries, timeouts, concurrency control, and HN item mapping into Article instances.[1]
- src/analyzer.rs: keyword automaton build, per-article scoring in parallel, and aggregation into ScoredArticle outputs.[1]
- src/model.rs: Article and related types with serde traits and encapsulated getters plus computed fields.[1]
- src/error.rs: AppError and Result<T> alias to unify error flows with thiserror.[1]
- src/config.rs: Config schema, defaults, validation, and Lazy global initialization.[1]
- src/metrics.rs: Arc<AtomicU64>-based counters and helpers for lightweight instrumentation.[1]
- src/rate_limiter.rs: governor-backed limiter type aliases and helpers for request pacing.[1]

Example tree (abridged) :[1]
```text
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

### Dependencies (core crates)[1]
- tokio: async runtime for non-blocking I/O and task scheduling.[1]
- reqwest: HTTP client with async support and JSON helpers.[1]
- rayon: data-parallel iterators and work-stealing thread pool.[1]
- aho-corasick: fast multi-pattern search for keyword matching.[1]
- serde + serde_json: serialization/deserialization for config and data.[1]
- thiserror: ergonomic error definitions and Display impls.[1]
- once_cell: thread-safe Lazy initialization of global Config.[1]
- governor: rate limiting to bound outbound request rate.[1]
- futures: stream utilities like buffer_unordered for concurrent pipelines.[1]
- num_cpus: determine optimal Rayon thread count defaults per host.[1]

### Sample output shape[1]
- Articles and scored results are serializable via serde and can be logged or exported as JSON depending on integration in main.rs.[1]
- Example JSON documents for downstream processing can include matched keyword lists and normalized scores alongside original article metadata.[1]

```json
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
This shows the general structure produced by combining Article with analyzer outputs for consumer-friendly pipelines and storage systems.[1]

### Development workflow[1]
- Use cargo fmt and cargo clippy to maintain quality and consistency across modules and ensure zero-cost abstractions remain well-optimized.[1]
- Split I/O-bound and CPU-bound work cleanly, and prefer Arc over Rc for any cross-thread sharing to satisfy Send/Sync and avoid data races.[1]

### Design notes[1]
- Encapsulation with private fields and public getters maintains invariants while providing a small, stable API surface for integration.[1]
- Trait-driven, generic utilities like fetch_with_retry<F, Fut, T> keep async workflows composable and testable without runtime overhead.[1]

### Roadmap ideas[1]
- Add CLI flags for output formatting, keyword management, and limits to complement config-based operation in diverse environments.[1]
- Expose an optional HTTP API or write-to-file mode for downstream systems that expect NDJSON or batched JSON artifacts.[1]

### Acknowledgements[1]
- Built as an educational and practical reference for safe, concurrent Rust systems that combine async I/O with parallel computation effectively.[1]
- Demonstrates compiler-enforced safety around ownership, borrowing, and lifetimes without sacrificing ergonomics or performance.[1]

### License[1]
- Choose and add a LICENSE file to clarify usage; MIT or Apache-2.0 are common for Rust libraries and tools of this nature.[1]

### Disclaimer[1]
- Ensure scraping adheres to target site policies and apply conservative rate limits to remain a good citizen of the web.[1]

Sources
[1] GitHub - amritsingh183/rst_news_aggregator: Use rust to scrape news from hackernews https://github.com/amritsingh183/rst_news_aggregator
[2] Office of the Chief Commissioner of CGST& Central ... http://etdut.gov.in/exciseonline/img/notifications/Taxpayers_Division.pdf
[3] NOTE It is hereby informed that all the cases which were ... https://highcourtchd.gov.in/show_cause_list.php?filename=MmVlZWU2MzJiNjkxZDVmMjM1NDRhMjhkOWViZDZiZWZUV3BCZVUweE9IZFBSamg1VDFZNU1WZ3lNSFZqUjFKdGJjNjU3YjU0MzFlNTRkNTJjN2NhYjg1ZTNmZGRmMzJi
[4] AMRIT YATRA - News Articles https://newindiasamachar.pib.gov.in/WriteReadData/Magazine/2022/Sep/M202209161.pdf
[5] MARITIME INDIA VISION 2030 http://sagarmala.gov.in/sites/default/files/MIV%202030%20Report.pdf
[6] 2022-23 https://www.meity.gov.in/static/uploads/2024/02/AR_2022-23_English_24-04-23-1.pdf
[7] takumade/rss-aggregator https://github.com/takumade/rss-aggregator
[8] Web Scraping With Rust – The Ultimate 2025 Guide https://iproyal.com/blog/web-scraping-with-rust-the-ultimate-guide/
[9] Public Works Department, Govt. of NCT of Delhi https://www.pwddelhi.gov.in/Home/ViewCircular
[10] RSS Aggregator - For RSS or HTML feeds https://github.com/hitem/rss-aggregator
[11] Build a Web Scraper in Rust and Deploy to Wasmer Edge https://wasmer.io/posts/news-scraper-on-edge
[12] annual report 2023-2024 https://www.msde.gov.in/static/uploads/2024/11/4f71465f72e9f90ff079f76ca2e374a9.pdf
[13] saiteja13427/News-Aggregator https://github.com/saiteja13427/News-Aggregator
[14] Web Scraping With Rust https://codeburst.io/web-scraping-in-rust-881b534a60f7?gi=83a1f4c7a6d5
[15] Economic Survey 2024-25 https://www.indiabudget.gov.in/economicsurvey/doc/echapter.pdf
[16] FreshRSS/FreshRSS: A free, self-hostable news ... https://github.com/FreshRSS/FreshRSS
[17] kxzk/scraping-with-rust https://github.com/kxzk/scraping-with-rust
[18] SL NO TRADE_NAME GSTIN_ID 1 M/S ANIL HARDWARE, ... https://tax.assam.gov.in/AssamTimsInfo/GST/ORDER-1-Annexure-A-CENTER-LIST.pdf
[19] rss-aggregator https://github.com/topics/rss-aggregator?l=php&o=asc&s=stars
[20] GitHub - alexheretic/hacker-news-scraper: Example Rust cli app. Scrapes posts from https://news.ycombinator.com/news into json. https://github.com/alexheretic/hacker-news-scraper
[21] News On AIR https://www.newsonair.gov.in
