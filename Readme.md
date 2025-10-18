# Comprehensive Rust Programming Guide( for all concepts used in this project)

### Based on a Real-World Article Aggregator Project


***

## Table of Contents

1. [Ownership and Memory Safety](#1-ownership-and-memory-safety)
2. [Borrowing and References](#2-borrowing-and-references)
3. [Lifetimes](#3-lifetimes)
4. [Structs and Implementation Blocks](#4-structs-and-implementation-blocks)
5. [Enums and Pattern Matching](#5-enums-and-pattern-matching)
6. [Error Handling with Result](#6-error-handling-with-result)
7. [Traits and Derive Macros](#7-traits-and-derive-macros)
8. [Generics and Trait Bounds](#8-generics-and-trait-bounds)
9. [Async/Await and Futures](#9-asyncawait-and-futures)
10. [Concurrency with Arc and Atomics](#10-concurrency-with-arc-and-atomics)
11. [Parallel Processing with Rayon](#11-parallel-processing-with-rayon)
12. [Closures and Function Traits](#12-closures-and-function-traits)
13. [Serialization with Serde](#13-serialization-with-serde)
14. [Module System and Visibility](#14-module-system-and-visibility)
15. [Advanced Patterns](#15-advanced-patterns)

***

## 1. Ownership and Memory Safety

### Concept

Rust's ownership system ensures memory safety without garbage collection. Every value has exactly one owner, and when the owner goes out of scope, the value is automatically dropped (deallocated).

**Three Rules of Ownership:**

1. Each value has exactly one owner
2. When the owner goes out of scope, the value is dropped
3. You can transfer ownership (move) or temporarily lend it (borrow)

### Example from Code

```rust
// src/model.rs
impl Article {
    pub fn new(title: String, url: String, source: String) -> Self {
        Self {
            title,      // Ownership of title String is MOVED into the struct
            url,        // Ownership of url String is MOVED into the struct
            source,     // Ownership of source String is MOVED into the struct
            description: None,
        }
    }
}
```

**Detailed Breakdown:**

```rust
// When you call Article::new:
let my_title = String::from("Rust Performance");
let my_url = String::from("https://example.com");
let my_source = String::from("Blog");

let article = Article::new(my_title, my_url, my_source);
// After this point, my_title, my_url, and my_source are NO LONGER VALID
// The ownership has been transferred to the Article struct
// println!("{}", my_title); // ❌ COMPILE ERROR: value borrowed after move
```


### Builder Pattern with Ownership

```rust
// src/model.rs
pub fn with_description(mut self, description: String) -> Self {
    //                   ^^^^^^^^
    // Takes ownership of self, modifies it, returns it
    self.description = Some(description);
    self  // Returns ownership back to caller
}
```

**Usage:**

```rust
// Chaining method calls (ownership transferred at each step)
let article = Article::new(
    "Title".to_string(),
    "URL".to_string(),
    "Source".to_string()
)
.with_description("Description text".to_string());  // Ownership moved in and out
```

**Why This Matters:**

- No memory leaks (automatic cleanup)
- No double-free errors
- No use-after-free bugs
- All checked at compile time with zero runtime cost!

***

## 2. Borrowing and References

### Concept

Borrowing allows you to reference data without taking ownership. There are two types:

- **Immutable borrows (`&T`)**: Multiple readers allowed
- **Mutable borrows (`&mut T`)**: Only one writer, no readers

**Borrowing Rules (enforced at compile time):**

1. You can have either ONE mutable reference OR any number of immutable references
2. References must always be valid (no dangling pointers)

### Example from Code

```rust
// src/analyzer.rs
impl ScoredArticle {
    pub fn article(&self) -> &Article {
    //             ^^^^^     ^^^^^^^^
    //             Borrows self   Returns borrowed reference
        &self.article
    }

    pub fn matched_keywords(&self) -> &[String] {
    //                                 ^^^^^^^^^
    //                                 Slice reference (borrowed)
        &self.matched_keywords
    }
}
```

**Detailed Breakdown:**

```rust
let scored = ScoredArticle { /* ... */ };

// Getting an immutable borrow
let article_ref = scored.article();  // Borrows the article (doesn't move it)
let title = article_ref.title();     // Can still use the reference

// Original scored is still valid!
let keywords = scored.matched_keywords();  // Can borrow again
println!("{:?}", scored);  // Still works - we never gave up ownership
```


### Immutable Borrowing in Functions

```rust
// src/analyzer.rs
fn calculate_relevance(
    article: &Article,        // Immutable borrow - read-only access
    ac: &AhoCorasick,        // Multiple immutable borrows are fine
    keywords: &[String],     // Slice borrow (also immutable)
) -> (f64, Vec<String>) {
    // We can read from article, ac, and keywords
    // But we CANNOT modify them
    
    let text = article.searchable_text().to_lowercase();
    //         ^^^^^^^^^^^^^^^^^^^^^^
    //         Borrows article temporarily
    
    // text is a NEW String (owned), not a borrow
    // This is fine because we're creating new data, not modifying borrowed data
    
    for mat in ac.find_iter(&text) {  // Borrow text for iteration
        // ...
    }
    
    (score, matched)  // Return owned data
}
```

**Real-World Usage:**

```rust
// src/analyzer.rs - Sharing AhoCorasick across parallel iterations
let ac = Arc::new(AhoCorasick::builder()...);

articles.into_par_iter().map(|article| {
    // Each parallel thread gets a borrowed reference via Arc
    let (score, matched) = calculate_relevance(&article, &ac, keywords);
    //                                         ^^^^^^^^  ^^^
    //                                         Borrows don't transfer ownership
    ScoredArticle { article, relevance_score: score, matched_keywords: matched }
})
```

**Why This Matters:**

- Read data without copying
- Multiple readers simultaneously
- Compiler prevents data races
- Zero runtime overhead

***

## 3. Lifetimes

### Concept

Lifetimes ensure that references are valid for as long as they're used. The compiler uses lifetime annotations to track how long references live.

### Example from Code

```rust
// src/analyzer.rs - Implicit lifetime annotations
impl ScoredArticle {
    pub fn matched_keywords(&self) -> &[String] {
        &self.matched_keywords
    }
}

// The compiler sees this as:
pub fn matched_keywords<'a>(&'a self) -> &'a [String] {
//                      ^^   ^^          ^^
//                      Lifetime parameter
    &self.matched_keywords
    // The returned slice lives as long as 'self' lives
}
```

**Detailed Breakdown:**

```rust
fn main() {
    let scored = ScoredArticle { /* ... */ };
    
    {
        let keywords = scored.matched_keywords();
        //   ^^^^^^^^
        //   This reference is valid only while 'scored' is valid
        println!("{:?}", keywords);
    } // keywords goes out of scope, but scored is still valid
    
    println!("{:?}", scored);  // Still works!
}
```


### Lifetime Elision Rules

Rust has three rules for inferring lifetimes (you don't need to write them explicitly):

1. Each parameter gets its own lifetime
2. If there's exactly one input lifetime, it's assigned to all output lifetimes
3. If there's a `&self` or `&mut self`, its lifetime is assigned to all outputs
```rust
// Written without explicit lifetimes (elided)
pub fn article(&self) -> &Article { &self.article }

// Compiler expands to:
pub fn article<'a>(&'a self) -> &'a Article { &self.article }
```

**Why This Matters:**

- Prevents dangling references at compile time
- No runtime cost
- Usually inferred automatically

***

## 4. Structs and Implementation Blocks

### Concept

Structs group related data together. Implementation blocks (`impl`) add methods to structs.

### Example from Code

```rust
// src/model.rs - Struct definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Article {
    title: String,           // Private field
    url: String,             // Private field
    source: String,          // Private field
    description: Option<String>,  // Optional field
}

// Implementation block - methods for Article
impl Article {
    // Constructor (associated function - no self)
    pub fn new(title: String, url: String, source: String) -> Self {
        Self {
            title,
            url,
            source,
            description: None,
        }
    }

    // Builder method (consumes and returns self)
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    // Getter methods (borrow self)
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    // Computed property
    pub fn searchable_text(&self) -> String {
        match &self.description {
            Some(desc) => format!("{} {}", self.title, desc),
            None => self.title.clone(),
        }
    }
}
```

**Detailed Breakdown:**

```rust
// Creating an article
let article = Article::new(
    "Rust Async".to_string(),
    "https://example.com".to_string(),
    "Blog".to_string()
);

// Adding description using builder pattern
let article = article.with_description("Great article about async Rust".to_string());

// Accessing fields through getters (encapsulation)
println!("Title: {}", article.title());  // ✅ Works
// println!("{}", article.title);        // ❌ Error: title is private

// Getting computed data
let searchable = article.searchable_text();  // Returns new String
```


### Another Example: ScoredArticle

```rust
// src/analyzer.rs
#[derive(Debug, Clone)]
pub struct ScoredArticle {
    article: Article,
    relevance_score: f64,
    matched_keywords: Vec<String>,
}

impl ScoredArticle {
    pub fn article(&self) -> &Article {
        &self.article
    }

    pub fn relevance_score(&self) -> f64 {
        self.relevance_score  // f64 is Copy, so this copies the value
    }

    pub fn matched_keywords(&self) -> &[String] {
        &self.matched_keywords  // Returns slice reference
    }
}
```

**Why This Matters:**

- Encapsulation (private fields, public methods)
- Type safety
- Clear API boundaries

***

## 5. Enums and Pattern Matching

### Concept

Enums represent a value that can be one of several variants. Pattern matching (`match`) handles each variant safely.

### Example from Code

```rust
// src/error.rs - Enum with different variant types
#[derive(Error, Debug, Clone)]
pub enum AppError {
    // Variant with named fields (struct-like)
    #[error("HTTP request failed for {url}: {message}")]
    HttpError { 
        url: String, 
        message: String 
    },

    // Variant with named fields
    #[error("Failed to parse response from {origin}: {message}")]
    ParseError { 
        origin: String, 
        message: String 
    },

    // Variant with single unnamed field (tuple-like)
    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    // Unit variant (no data)
    #[error("Shutdown requested")]
    ShutdownError,

    #[error("No articles found from source: {0}")]
    NoArticlesError(String),

    #[error("Analyzer error: {0}")]
    AnalyzerError(String),
}
```

**Creating Enum Values:**

```rust
// Creating different variants
let http_err = AppError::HttpError {
    url: "https://example.com".to_string(),
    message: "Connection timeout".to_string(),
};

let timeout_err = AppError::TimeoutError("Request took too long".to_string());

let shutdown = AppError::ShutdownError;
```


### Pattern Matching on Enums

```rust
// src/main.rs - Matching on Result<T, AppError>
match result {
    Ok(article) => {
        // Success case - we have an article
        articles.push(article);
        metrics.record_article_fetched();
    }
    Err(e) => {
        // Error case - log and record failure
        warn!(source = source, error = %e, "Failed to fetch article");
        metrics.record_article_failed();
    }
}
```


### Exhaustive Matching

```rust
// Compiler ensures all variants are handled
match error {
    AppError::HttpError { url, message } => {
        println!("HTTP error at {}: {}", url, message);
    }
    AppError::ParseError { origin, message } => {
        println!("Parse error from {}: {}", origin, message);
    }
    AppError::TimeoutError(msg) => {
        println!("Timeout: {}", msg);
    }
    AppError::ShutdownError => {
        println!("Shutting down gracefully");
    }
    AppError::RateLimitError(msg) => {
        println!("Rate limited: {}", msg);
    }
    AppError::ConfigError(msg) => {
        println!("Config error: {}", msg);
    }
    AppError::NoArticlesError(source) => {
        println!("No articles from: {}", source);
    }
    AppError::AnalyzerError(msg) => {
        println!("Analyzer error: {}", msg);
    }
    // If you forget a variant, the compiler will error!
}
```


### Option Enum Pattern Matching

```rust
// src/model.rs
pub fn searchable_text(&self) -> String {
    match &self.description {
        Some(desc) => format!("{} {}", self.title, desc),  // Has description
        None => self.title.clone(),  // No description
    }
}
```

**Using if let for Single Pattern:**

```rust
// src/fetcher.rs - Only care about Some variant
if let Some(href) = link.value().attr("href") {
    // Only executes if href is Some
    let url = if href.starts_with("http") {
        href.to_string()
    } else {
        format!("{}{}", BLOG_URL.trim_end_matches('/'), href)
    };
}
```

**Why This Matters:**

- Compile-time exhaustiveness checking
- No null pointer exceptions
- Explicit error handling
- Type-safe variants

***

## 6. Error Handling with Result

### Concept

Rust uses `Result<T, E>` for operations that can fail. It's an enum with two variants: `Ok(T)` for success and `Err(E)` for failure.

### Example from Code

```rust
// src/error.rs - Type alias for cleaner signatures
pub type Result<T> = std::result::Result<T, AppError>;
//                   ^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^
//                   Standard Result     Our error type

// Now instead of writing:
// fn fetch() -> std::result::Result<Vec<Article>, AppError>

// We can write:
// fn fetch() -> Result<Vec<Article>>
```


### The ? Operator (Early Return on Error)

```rust
// src/config.rs
impl Config {
    pub fn load() -> Result<Self> {
        let config = ConfigBuilder::builder()
            .add_source(File::with_name("config").required(false))
            .add_source(Environment::with_prefix("APP"))
            .build()
            .map_err(|e| AppError::ConfigError(e.to_string()))?;
        //                                                     ^
        // If build() returns Err, convert it to AppError and return early
        // If Ok, unwrap the value and continue

        let cfg: Self = config
            .try_deserialize()
            .map_err(|e| AppError::ConfigError(e.to_string()))?;
        //                                                     ^
        // Another early return if deserialization fails

        cfg.validate()?;  // If validate returns Err, propagate it
        //           ^
        // Short form when error types already match

        Ok(cfg)  // Success - return the config wrapped in Ok
    }
}
```

**Without ? Operator (verbose):**

```rust
// What ? operator replaces
pub fn load() -> Result<Self> {
    let config = match ConfigBuilder::builder()
        .add_source(File::with_name("config").required(false))
        .add_source(Environment::with_prefix("APP"))
        .build() 
    {
        Ok(c) => c,
        Err(e) => return Err(AppError::ConfigError(e.to_string())),
    };

    let cfg: Self = match config.try_deserialize() {
        Ok(c) => c,
        Err(e) => return Err(AppError::ConfigError(e.to_string())),
    };

    match cfg.validate() {
        Ok(()) => {},
        Err(e) => return Err(e),
    }

    Ok(cfg)
}
```


### Error Conversion with map_err

```rust
// src/main.rs
let client = Client::builder()
    .timeout(CONFIG.http_timeout())
    .pool_max_idle_per_host(CONFIG.http.pool_max_idle_per_host)
    .user_agent("article-aggregator/0.1.0")
    .build()
    .map_err(|e| AppError::http_error("client", e))?;
    //^^^^^^^
    // Converts reqwest::Error to AppError
```


### Double ? Operator Pattern

```rust
// src/main.rs - Handling nested Results
let scored_articles = tokio::task::spawn_blocking(move || {
    analyzer::score_articles(articles, &keywords)
    // Returns: Result<Vec<ScoredArticle>, AppError>
})
.await  // Returns: Result<Result<Vec<ScoredArticle>, AppError>, JoinError>
?       // First ? handles JoinError (task panic)
?;      // Second ? handles AppError (analyzer error)

// Equivalent to:
let join_result = tokio::task::spawn_blocking(move || {
    analyzer::score_articles(articles, &keywords)
}).await;

let task_result = match join_result {
    Ok(inner) => inner,  // Got Result<Vec<ScoredArticle>, AppError>
    Err(join_error) => return Err(AppError::parse_error("analyzer", join_error)),
};

let scored_articles = match task_result {
    Ok(articles) => articles,
    Err(app_error) => return Err(app_error),
};
```


### Result Helper Methods

```rust
// src/config.rs
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    Config::load().unwrap_or_else(|_| {
    //             ^^^^^^^^^^^^^^
    // If Ok, unwrap value
    // If Err, call closure and use its return value
        eprintln!("Failed to load config, using defaults");
        Config::default()
    })
});
```

**Why This Matters:**

- Explicit error handling (no hidden exceptions)
- Type-safe error propagation
- Composable error handling
- ? operator makes it ergonomic

***

## 7. Traits and Derive Macros

### Concept

Traits define shared behavior across types (like interfaces in other languages). Derive macros automatically implement common traits.

### Example from Code

```rust
// src/model.rs
#[derive(Debug, Clone, Deserialize, Serialize)]
//       ^^^^^  ^^^^^  ^^^^^^^^^^^  ^^^^^^^^^
//       Trait implementations generated automatically
pub struct Article {
    title: String,
    url: String,
    source: String,
    description: Option<String>,
}
```

**What Each Derive Does:**

1. **Debug** - Enables `{:?}` formatting for debugging:
```rust
let article = Article::new(...);
println!("{:?}", article);
// Output: Article { title: "...", url: "...", source: "...", description: None }
```

2. **Clone** - Enables explicit copying:
```rust
let article1 = Article::new(...);
let article2 = article1.clone();  // Deep copy
// Both article1 and article2 are now valid
```

3. **Deserialize** - Enables parsing from JSON/TOML/etc:
```rust
let json = r#"{"title":"Rust","url":"https://...","source":"Blog"}"#;
let article: Article = serde_json::from_str(json)?;
```

4. **Serialize** - Enables converting to JSON/TOML/etc:
```rust
let article = Article::new(...);
let json = serde_json::to_string(&article)?;
```


### Manual Trait Implementation

```rust
// src/config.rs - Implementing Default trait manually
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
                rayon_threads: num_cpus::get(),
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

// Usage:
let config = Config::default();
```


### Custom Error Trait with thiserror

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug, Clone)]
//       ^^^^^
//       Implements std::error::Error trait
pub enum AppError {
    #[error("HTTP request failed for {url}: {message}")]
    //      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    //      Implements Display trait with this format string
    HttpError { url: String, message: String },

    #[error("Timeout error: {0}")]
    //                      ^^^
    //                      {0} refers to first tuple field
    TimeoutError(String),
}

// Now you can use AppError like any standard error:
fn handle_error(e: AppError) {
    println!("Error: {}", e);  // Uses Display implementation
    println!("Debug: {:?}", e);  // Uses Debug implementation
}
```


### Trait Bounds on Generics

```rust
// src/error.rs
impl AppError {
    pub fn http_error(url: impl Into<String>, err: impl std::fmt::Display) -> Self {
    //                     ^^^^^^^^^^^^^^^^        ^^^^^^^^^^^^^^^^^^^^^^
    //                     Any type that can        Any type that can be
    //                     convert into String      formatted for display
        Self::HttpError {
            url: url.into(),  // Converts to String
            message: err.to_string(),  // Uses Display trait
        }
    }
}

// Usage:
let error = AppError::http_error("https://example.com", "Connection refused");
//                                ^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^
//                                &str implements Into<String>
//                                &str implements Display
```

**Why This Matters:**

- Code reuse (don't reimplement common behavior)
- Polymorphism (treat different types uniformly)
- Automatic implementations with derive
- Compile-time polymorphism (zero cost)

***

## 8. Generics and Trait Bounds

### Concept

Generics allow writing code that works with multiple types. Trait bounds constrain which types can be used.

### Example from Code

```rust
// src/fetcher.rs
async fn fetch_with_retry<F, Fut, T>(&self, url: &str, operation: F) -> Result<T>
//                        ^^^^^^^^^^
//                        Generic type parameters
where
    F: Fn() -> Fut,
    //  ^^^^^^^^^^
    // F must be a function that returns Fut
    
    Fut: std::future::Future<Output = Result<T>>,
    //   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    // Fut must be a Future that outputs Result<T>
{
    // Function body can work with ANY types F, Fut, T
    // as long as they satisfy the trait bounds
}
```

**Detailed Breakdown:**

```rust
// Generic parameters:
// F   = Type of the operation (closure/function)
// Fut = Type of the Future returned by F
// T   = Type of the successful result

// Example usage:
self.fetch_with_retry(url, || async {
//                     ^^^^^^^^^^^^^
//  F = closure type (implements Fn() -> Fut)
//  Fut = impl Future<Output = Result<Vec<u64>>>
//  T = Vec<u64>
    let response = self.client.get(url).send().await?;
    let ids: Vec<u64> = response.json().await?;
    Ok(ids)
}).await?;
```


### How It Works

```rust
// The compiler generates specialized versions for each concrete type:

// When called with operation returning Future<Output = Result<Vec<u64>>>:
async fn fetch_with_retry_specialized(
    &self,
    url: &str,
    operation: impl Fn() -> impl Future<Output = Result<Vec<u64>>>
) -> Result<Vec<u64>> {
    // ... specialized code ...
}

// When called with operation returning Future<Output = Result<String>>:
async fn fetch_with_retry_specialized_2(
    &self,
    url: &str,
    operation: impl Fn() -> impl Future<Output = Result<String>>
) -> Result<String> {
    // ... specialized code ...
}

// All at compile time - zero runtime cost!
```


### Real Usage in Code

```rust
// src/fetcher.rs
let story_ids: Vec<u64> = match self
    .fetch_with_retry(url, || async {
    //                ^^^^^^^^^^
    //  F = closure type
        let response = self.client.get(url).send().await
            .map_err(|e| AppError::http_error(url, e))?;
        //           ^^^
        // Fut = Future that returns Result<Response>

        response
            .error_for_status()
            .map_err(|e| AppError::http_error(url, e))?
            .json()
            .await
            .map_err(|e| AppError::parse_error("HackerNews", e))
        // T = Vec<u64> (inferred from .json() call)
    })
    .await
{
    Ok(ids) => ids,
    Err(e) => return vec![Err(e)],
};
```


### Generic Structs

```rust
// Standard library Result is generic:
pub enum Result<T, E> {
    Ok(T),   // Success with value of type T
    Err(E),  // Failure with error of type E
}

// Our type alias:
pub type Result<T> = std::result::Result<T, AppError>;
//       ^^^^^^                           ^^  ^^^^^^^^
//       Still generic in T              Fixed  Fixed E type
```

**Why This Matters:**

- Write code once, use with many types
- Type safety maintained
- No runtime overhead (monomorphization)
- Compiler generates optimized code for each type

***

## 9. Async/Await and Futures

### Concept

`async/await` enables writing asynchronous code that looks synchronous. Functions marked `async` return `Future`s that represent values available later.

### Example from Code

```rust
// src/fetcher.rs
pub async fn fetch_hacker_news(&self) -> Vec<Result<Article>> {
//     ^^^^^
// async keyword makes this function return impl Future<Output = Vec<Result<Article>>>

    let url = "https://hacker-news.firebaseio.com/v0/topstories.json";
    
    // Await suspends this function until the future completes
    let story_ids: Vec<u64> = match self.fetch_with_retry(...).await {
    //                                                          ^^^^^
    // .await yields control back to runtime until ready
        Ok(ids) => ids,
        Err(e) => return vec![Err(e)],
    };

    // More awaits...
}
```

**What Happens Under the Hood:**

```rust
// This async function:
async fn fetch_data() -> String {
    let response = http_get("https://api.example.com").await;
    response.text().await
}

// Actually returns a Future (state machine):
fn fetch_data() -> impl Future<Output = String> {
    async {
        let response = http_get("https://api.example.com").await;
        response.text().await
    }
}

// The runtime polls this Future:
loop {
    match future.poll() {
        Poll::Ready(value) => return value,
        Poll::Pending => {
            // Register wakeup and return to runtime
            // Runtime will poll again when data arrives
        }
    }
}
```


### Tokio Runtime

```rust
// src/main.rs
#[tokio::main(flavor = "multi_thread")]
//^^^^^^^^^^^^
// Macro that creates async runtime
async fn main() -> Result<()> {
    // This is now an async function
    // Can use .await inside
}

// Macro expands to something like:
fn main() -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async_main())
}

async fn async_main() -> Result<()> {
    // Your actual main code
}
```


### Concurrent Execution with tokio::join!

```rust
// src/main.rs
let (hn_results, blog_results) = tokio::join!(
    //                           ^^^^^^^^^^^^
    // Runs both futures concurrently
    fetcher.fetch_hacker_news(),
    fetcher.fetch_rust_blog()
);

// Without tokio::join! (sequential - slower):
let hn_results = fetcher.fetch_hacker_news().await;
let blog_results = fetcher.fetch_rust_blog().await;

// With tokio::join! (concurrent - faster):
// Both fetches run at the same time!
```


### Stream Processing

```rust
// src/fetcher.rs
use futures::stream::{self, StreamExt};

let futures = story_ids.into_iter().map(|id| {
    async move {
        // Each id gets its own async block (Future)
        let url = format!("https://hacker-news.firebaseio.com/v0/item/{}.json", id);
        let response = client.get(&url).send().await?;
        let item: HackerNewsItem = response.json().await?;
        Ok(Article::new(item.title, item.url.unwrap_or_default(), "HN".to_string()))
    }
});

// Execute up to 10 futures concurrently
stream::iter(futures)
    .buffer_unordered(10)
    //^^^^^^^^^^^^^^^^
    // Polls up to 10 futures at once
    // Collects results as they complete (unordered)
    .collect::<Vec<_>>()
    .await
```

**Visual of buffer_unordered:**

```
Futures: [F1] [F2] [F3] [F4] [F5] [F6] [F7] [F8] [F9] [F10] [F11] [F12]

With buffer_unordered(3):
Time 0: Poll F1, F2, F3
Time 1: F2 completes → Poll F4
Time 2: F1 completes → Poll F5
Time 3: F4 completes → Poll F6
...

Results collected as they complete (order may vary)
```


### Timeouts and Retry Logic

```rust
// src/fetcher.rs
match timeout(CONFIG.http_timeout(), operation()).await {
//    ^^^^^^^                                     ^^^^^
//    Wraps future with timeout   Actually executes the future
    
    Ok(Ok(result)) => return Ok(result),
    //  ^^  Success from timeout    ^^ Success from operation
    
    Ok(Err(e)) if attempts >= max_attempts => {
        // Timeout succeeded, but operation failed
        return Err(e);
    }
    
    Err(_) => {
        // Timeout expired
        if attempts >= max_attempts {
            return Err(AppError::TimeoutError(...));
        }
    }
}
```

**Why This Matters:**

- Write async code that looks synchronous
- Efficient I/O (thousands of concurrent operations)
- Non-blocking execution
- Composable async operations

***

## 10. Concurrency with Arc and Atomics

### Concept

`Arc` (Atomic Reference Counting) enables sharing data across threads safely. Atomics provide lock-free synchronization.

### Example from Code: Arc for Sharing

```rust
// src/analyzer.rs
pub fn score_articles(articles: Vec<Article>, keywords: &[String]) -> Result<Vec<ScoredArticle>> {
    let ac = AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .build(&patterns)?;
    
    let ac = Arc::new(ac);
    //       ^^^^^^^^
    // Wrap in Arc to share across threads

    articles.into_par_iter().map(|article| {
        // Each thread gets a clone of the Arc
        // Arc::clone is cheap (just increments atomic counter)
        let (score, matched) = calculate_relevance(&article, &ac, keywords);
        //                                                    ^^^
        // All threads share the same AhoCorasick automaton
        ScoredArticle { article, relevance_score: score, matched_keywords: matched }
    }).collect()
}
```

**How Arc Works:**

```rust
struct Arc<T> {
    ptr: *const ArcInner<T>,  // Pointer to heap-allocated data
}

struct ArcInner<T> {
    strong_count: AtomicUsize,  // Reference counter (atomic!)
    data: T,                     // The actual data
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        // Atomically increment reference count
        self.inner().strong_count.fetch_add(1, Ordering::Relaxed);
        Arc { ptr: self.ptr }  // Copy pointer (cheap!)
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        // Atomically decrement reference count
        if self.inner().strong_count.fetch_sub(1, Ordering::Release) == 1 {
            // Last reference - deallocate
            unsafe { drop(Box::from_raw(self.ptr)) }
        }
    }
}
```


### Example from Code: AtomicU64 for Metrics

```rust
// src/metrics.rs
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct Metrics {
    articles_fetched: Arc<AtomicU64>,
    //                    ^^^^^^^^^^
    // Atomic counter - lock-free!
    articles_failed: Arc<AtomicU64>,
    http_requests: Arc<AtomicU64>,
    http_failures: Arc<AtomicU64>,
}

impl Metrics {
    pub fn record_article_fetched(&self) {
        self.articles_fetched.fetch_add(1, Ordering::Relaxed);
        //                    ^^^^^^^^^
        // Atomically increment by 1
        //                              ^^^^^^^^^^^^^^^^
        // Memory ordering (Relaxed = no synchronization)
    }
}
```

**Memory Ordering Explained:**

```rust
// Ordering::Relaxed
// - Only guarantees atomicity of this operation
// - No ordering with respect to other operations
// - Fastest option
// - Use for independent counters

self.counter.fetch_add(1, Ordering::Relaxed);

// Ordering::Acquire
// - Prevents reordering of subsequent reads
// - Use when loading data that must be synchronized

let value = self.flag.load(Ordering::Acquire);
// All reads after this see updates before the store

// Ordering::Release
// - Prevents reordering of previous writes
// - Use when storing data that must be synchronized

self.flag.store(true, Ordering::Release);
// All writes before this are visible after the load

// Ordering::SeqCst
// - Sequentially consistent (strongest)
// - Total order of all SeqCst operations
// - Slowest but simplest to reason about
// - Use when you need strong ordering guarantees
```


### Real Usage Pattern

```rust
// src/fetcher.rs
let futures = story_ids.into_iter().map(|id| {
    let rate_limiter = self.rate_limiter.clone();
    //                 ^^^^^^^^^^^^^^^^^^^^^^^^^^
    // Clone the Arc - cheap! Just increments counter
    
    let metrics = self.metrics.clone();
    //            ^^^^^^^^^^^^^^^^^^^^
    // Another Arc clone
    
    async move {
        rate_limiter.wait().await?;
        //^^^^^^^^^^
        // Shared reference to rate limiter
        
        metrics.record_http_request();
        //^^^^^
        // Shared reference to metrics
        
        // ... fetch data ...
    }
});
```


### Why Arc Instead of Rc?

```rust
// Rc (Reference Counted) - NOT thread-safe
use std::rc::Rc;

let rc = Rc::new(5);
// ❌ Cannot send to another thread
// thread::spawn(move || { println!("{}", rc); });  // Compile error!

// Arc (Atomic Reference Counted) - Thread-safe
use std::sync::Arc;

let arc = Arc::new(5);
// ✅ Can send to another thread
thread::spawn(move || { println!("{}", arc); });  // Works!
```

**Performance Comparison:**

```rust
// Rc::clone - Fast (single-threaded)
let rc2 = Rc::clone(&rc);  // Just increments counter

// Arc::clone - Slightly slower (uses atomic operations)
let arc2 = Arc::clone(&arc);  // Atomic increment (sync across cores)

// But Arc enables parallelism!
// Parallel speedup >> atomic overhead
```

**Why This Matters:**

- Safe concurrent data sharing
- No data races (enforced by compiler)
- Lock-free counters (better performance than mutexes)
- Enables efficient parallelism

***

## 11. Parallel Processing with Rayon

### Concept

Rayon provides data parallelism through parallel iterators that automatically distribute work across CPU cores.

### Example from Code

```rust
// src/analyzer.rs
use rayon::prelude::*;

pub fn score_articles(articles: Vec<Article>, keywords: &[String]) -> Result<Vec<ScoredArticle>> {
    let ac = Arc::new(AhoCorasick::builder()...);

    let scored = articles
        .into_par_iter()
        //^^^^^^^^^^^^^^
        // Convert to parallel iterator - work split across threads!
        
        .map(|article| {
        //^^^
        // Each article processed in parallel
            let (score, matched) = calculate_relevance(&article, &ac, keywords);
            ScoredArticle {
                article,
                relevance_score: score,
                matched_keywords: matched,
            }
        })
        .collect();
        //^^^^^^^
        // Parallel collection back to Vec

    Ok(scored)
}
```

**Visual Representation:**

```
Sequential (.iter()):
Thread: [A1] → [A2] → [A3] → [A4] → [A5] → [A6] → [A7] → [A8]
Time: ========================================>

Parallel (.into_par_iter() with 4 threads):
Thread 1: [A1] → [A5]
Thread 2: [A2] → [A6]
Thread 3: [A3] → [A7]
Thread 4: [A4] → [A8]
Time: ==========>

~4x speedup on 4 cores!
```


### Thread Pool Configuration

```rust
// src/analyzer.rs
pub fn init_rayon_pool() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(CONFIG.analyzer.rayon_threads)
        //           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        // Number of worker threads (usually = num_cpus)
        
        .thread_name(|i| format!("rayon-worker-{}", i))
        //           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        // Names threads for debugging (helpful in profilers!)
        
        .build_global()
        //^^^^^^^^^^^^
        // Sets as the global default thread pool
        
        .expect("Failed to build Rayon thread pool");
}

// src/config.rs
analyzer: AnalyzerConfig {
    rayon_threads: num_cpus::get(),  // Dynamic based on hardware
}
```


### Work Stealing

Rayon uses a **work-stealing scheduler**:

```
Initial distribution (8 tasks, 4 threads):
Thread 1: [T1, T2]
Thread 2: [T3, T4]
Thread 3: [T5, T6]
Thread 4: [T7, T8]

Thread 4 finishes T7, T8 quickly, so it "steals" from Thread 1:
Thread 1: [T1] (still working)
Thread 2: [T3, T4]
Thread 3: [T5, T6]
Thread 4: [T7, T8, T2] ← Stole T2 from Thread 1!

Result: Better load balancing, less idle time
```


### Parallel Iterator Methods

```rust
// Filter in parallel
let filtered: Vec<_> = data
    .into_par_iter()
    .filter(|x| x.score > 10.0)
    .collect();

// Map and reduce in parallel
let sum: f64 = data
    .par_iter()
    .map(|x| x.value * 2.0)
    .sum();

// Find in parallel (stops early when found)
let result = data
    .par_iter()
    .find_any(|x| x.id == target_id);

// Sort in parallel
let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
data.par_sort();  // Parallel quicksort
```


### Rayon vs Tokio

```rust
// Tokio: For I/O-bound work (network, files)
async fn fetch_urls(urls: Vec<String>) -> Vec<Response> {
    let futures = urls.into_iter().map(|url| {
        async move {
            reqwest::get(&url).await  // Waits for network
        }
    });
    
    futures::future::join_all(futures).await
    // While waiting for network, CPU is free for other tasks
}

// Rayon: For CPU-bound work (computation, parsing)
fn process_data(data: Vec<Data>) -> Vec<Result> {
    data.into_par_iter().map(|item| {
        expensive_calculation(item)  // Uses CPU intensively
    }).collect()
    // Distributes computation across CPU cores
}
```


### Bridging Async and Parallel

```rust
// src/main.rs
// Fetch articles asynchronously (I/O-bound)
let articles = fetcher.fetch_hacker_news().await;

// Process articles in parallel (CPU-bound)
let scored_articles = tokio::task::spawn_blocking(move || {
    //                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    // Runs Rayon work on separate thread pool
    analyzer::score_articles(articles, &keywords)
}).await??;

// Best of both worlds:
// - Async for efficient I/O
// - Rayon for efficient computation
```

**Why This Matters:**

- Automatic parallelization (no manual thread management)
- Work stealing for load balancing
- Safe sharing with Arc
- Significant speedups on multi-core CPUs

***

## 12. Closures and Function Traits

### Concept

Closures are anonymous functions that can capture variables from their environment. Rust has three closure traits: `Fn`, `FnMut`, and `FnOnce`.

### Example from Code

```rust
// src/fetcher.rs
let futures = story_ids.into_iter().map(|id| {
//                                      ^^^^
// Closure parameter
    let client = self.client.clone();
    let rate_limiter = self.rate_limiter.clone();
    //  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    // Capturing variables from outer scope
    
    async move {
    //    ^^^^
    // Takes ownership of captured variables
        rate_limiter.wait().await?;
        let response = client.get(&url).send().await?;
        // ...
    }
});
```


### Closure Syntax

```rust
// Various closure forms:

// No parameters, no captures
let f = || println!("Hello");

// One parameter
let add_one = |x| x + 1;

// Multiple parameters with type annotations
let multiply = |x: i32, y: i32| -> i32 { x * y };

// Multiple statements
let process = |text: &str| {
    let lower = text.to_lowercase();
    let trimmed = lower.trim();
    trimmed.to_string()
};

// Capturing environment
let threshold = 10;
let is_above = |x: i32| x > threshold;  // Captures 'threshold'
```


### The Three Function Traits

```rust
// FnOnce - Can be called once, consumes captured values
fn call_once<F>(f: F) where F: FnOnce() {
    f();  // Can only call once
}

let data = String::from("hello");
let consume = || {
    drop(data);  // Consumes data
};
call_once(consume);
// consume();  // ❌ Error: already called

// FnMut - Can be called multiple times, mutates captured values
fn call_many<F>(mut f: F) where F: FnMut() {
    f();
    f();  // Can call multiple times
}

let mut counter = 0;
let mut increment = || {
    counter += 1;  // Mutates counter
};
call_many(&mut increment);

// Fn - Can be called multiple times, immutably borrows
fn call_many_ref<F>(f: &F) where F: Fn() {
    f();
    f();  // Can call many times
}

let message = "hello";
let print = || {
    println!("{}", message);  // Only reads message
};
call_many_ref(&print);
```

**Trait Hierarchy:**

```
FnOnce
  ↑
FnMut (also implements FnOnce)
  ↑
Fn (also implements FnMut and FnOnce)

Fn is the most restrictive (most functions satisfy it)
FnOnce is the least restrictive (all closures satisfy it)
```


### Example: fetch_with_retry

```rust
// src/fetcher.rs
async fn fetch_with_retry<F, Fut, T>(&self, url: &str, operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    //  ^^
    // operation can be called multiple times (for retries!)
{
    loop {
        match timeout(CONFIG.http_timeout(), operation()).await {
        //                                   ^^^^^^^^^^^
        // Calling the closure (could be called multiple times)
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(e)) if attempts >= max_attempts => return Err(e),
            Err(_) if attempts >= max_attempts => return Err(...),
            _ => {
                // Retry - will call operation() again
            }
        }
    }
}
```


### move Keyword

```rust
// Without move - borrows
let data = vec![1, 2, 3];
let printer = || println!("{:?}", data);
//            ^^
// Borrows data
printer();
println!("{:?}", data);  // ✅ Still valid

// With move - takes ownership
let data = vec![1, 2, 3];
let printer = move || println!("{:?}", data);
//            ^^^^
// Takes ownership of data
printer();
// println!("{:?}", data);  // ❌ Error: value moved

// Useful for spawning threads/tasks
let data = vec![1, 2, 3];
tokio::spawn(async move {
//                 ^^^^
// Moves data into async block
    process(data).await
});
// data is no longer valid here
```


### Real-World Pattern

```rust
// src/main.rs
tokio::spawn(async move {
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutdown signal received");
            shutdown_token.cancel();
            //^^^^^^^^^^^^^
            // Can use shutdown_token because it was moved in
        }
        Err(err) => {
            error!(error = %err, "Failed to listen");
        }
    }
});
```

**Why This Matters:**

- Inline anonymous functions
- Capture environment variables
- Type inference (less boilerplate)
- Enables functional programming patterns
- Powers iterators and async code

***

## 13. Serialization with Serde

### Concept

Serde is Rust's serialization framework. It can convert data structures to/from JSON, TOML, YAML, etc.

### Example from Code

```rust
// src/model.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
//                     ^^^^^^^^^^^  ^^^^^^^^^
// Deserialize: Parse from JSON/TOML → Rust struct
// Serialize: Convert Rust struct → JSON/TOML
pub struct Article {
    title: String,
    url: String,
    source: String,
    description: Option<String>,
}
```

**Serialization Example:**

```rust
use serde_json;

let article = Article::new(
    "Rust Async".to_string(),
    "https://example.com".to_string(),
    "Blog".to_string()
);

// Convert to JSON string
let json = serde_json::to_string(&article).unwrap();
println!("{}", json);
// Output: {"title":"Rust Async","url":"https://example.com","source":"Blog","description":null}

// Convert to pretty JSON
let json_pretty = serde_json::to_string_pretty(&article).unwrap();
println!("{}", json_pretty);
// Output:
// {
//   "title": "Rust Async",
//   "url": "https://example.com",
//   "source": "Blog",
//   "description": null
// }
```

**Deserialization Example:**

```rust
let json = r#"{
    "title": "Rust Async",
    "url": "https://example.com",
    "source": "Blog",
    "description": "A great article"
}"#;

let article: Article = serde_json::from_str(json).unwrap();
println!("{}", article.title());  // "Rust Async"
```


### Field Attributes

```rust
// src/model.rs
#[derive(Deserialize)]
pub struct HackerNewsItem {
    pub id: u64,
    pub title: String,
    
    #[serde(default)]
    //      ^^^^^^^
    // If field is missing in JSON, use Default::default()
    pub url: Option<String>,
    
    #[serde(default)]
    pub text: Option<String>,
}
```

**How \#[serde(default)] Works:**

```rust
// JSON with all fields
let json1 = r#"{"id":123,"title":"Test","url":"https://...","text":"Body"}"#;
let item1: HackerNewsItem = serde_json::from_str(json1)?;
// item1.url = Some("https://...")
// item1.text = Some("Body")

// JSON with missing optional fields
let json2 = r#"{"id":123,"title":"Test"}"#;
let item2: HackerNewsItem = serde_json::from_str(json2)?;
// item2.url = None (Serde treats Option<T> specially - missing = None)
// item2.text = None (even without #[serde(default)])

// Note: Option<T> fields automatically default to None when missing.
// #[serde(default)] is needed for other types like Vec<T>, String, etc.
```

**Example where #[serde(default)] is actually needed:**

```rust
#[derive(Deserialize)]
pub struct Config {
    pub name: String, // Required field
    #[serde(default)]
    pub tags: Vec<String>,      // Defaults to empty vec if missing
    #[serde(default = "default_timeout")]
    pub timeout: u64,           // Custom default if missing
}

fn default_timeout() -> u64 { 30 }
// Without #[serde(default)] on tags:
// {"name":"test"} would ERROR because tags is missing

// With #[serde(default)] on tags:
// {"name":"test"} → Config { name: "test", tags: vec![], timeout: 30 }
```
### Configuration Deserialization

```rust
// src/config.rs
use config::{Config as ConfigBuilder, Environment, File};

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
            //          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
            // Load from config.toml, config.json, etc.
            
            .add_source(Environment::with_prefix("APP"))
            //          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
            // Override with environment variables (APP_HTTP_TIMEOUT_SECS, etc.)
            
            .build()?;

        let cfg: Self = config.try_deserialize()?;
        //              ^^^^^^^^^^^^^^^^^^^^^^
        // Parse into our Config struct
        
        cfg.validate()?;
        Ok(cfg)
    }
}
```

**Configuration File Example (config.toml):**

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

**Environment Variable Override:**

```bash
# Override specific values
export APP_HTTP_TIMEOUT_SECS=20
export APP_FETCHER_MAX_CONCURRENT_REQUESTS=50
export APP_KEYWORDS_VALUES="rust,async,web"

# Run program - environment vars take precedence
cargo run
```


### Custom Serialization

```rust
// Custom serialization format
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
struct KeywordsConfig {
    #[serde(deserialize_with = "deserialize_keywords")]
    values: Vec<String>,
}

fn deserialize_keywords<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    Ok(s.split(',')
         .map(|s| s.trim().to_lowercase())
         .collect())
}

// Now can parse "rust, async, tokio" → vec!["rust", "async", "tokio"]
```

**Why This Matters:**

- Type-safe serialization
- Works with multiple formats (JSON, TOML, YAML, etc.)
- Automatic validation
- Minimal boilerplate
- Excellent error messages

***

## 14. Module System and Visibility

### Concept

Rust organizes code into modules. Items are private by default; use `pub` to expose them.

### Example from Code

```rust
// src/main.rs
mod analyzer;     // Loads src/analyzer.rs
mod config;       // Loads src/config.rs
mod error;        // Loads src/error.rs
mod fetcher;      // Loads src/fetcher.rs
mod metrics;      // Loads src/metrics.rs
mod model;        // Loads src/model.rs
mod rate_limiter; // Loads src/rate_limiter.rs

// Use statements bring items into scope
use crate::analyzer::ScoredArticle;
//    ^^^^^
// crate = root of current crate
use crate::config::CONFIG;
use crate::error::{AppError, Result};
use crate::fetcher::Fetcher;
use crate::metrics::Metrics;
```


### Visibility Rules

```rust
// src/model.rs
pub struct Article {
//  ^^^
// Public struct - visible outside module
    title: String,          // Private field
    url: String,            // Private field
    source: String,         // Private field
    description: Option<String>,  // Private field
}

impl Article {
    pub fn new(title: String, url: String, source: String) -> Self {
    //  ^^^
    // Public function - part of public API
        Self { title, url, source, description: None }
    }

    pub fn title(&self) -> &str {
    //  ^^^
    // Public getter
        &self.title  // Can access private field within module
    }
    
    fn internal_helper(&self) {
    // ^^^ No pub = private function
        // Only visible within this module
    }
}
```

**Access from Outside:**

```rust
// src/main.rs
use crate::model::Article;

let article = Article::new(
    "Title".to_string(),
    "URL".to_string(),
    "Source".to_string()
);

println!("{}", article.title());  // ✅ Public getter
// println!("{}", article.title); // ❌ Private field
```


### Nested Modules

```rust
// src/lib.rs (library root)
pub mod network {
    pub mod http {
        pub fn get(url: &str) -> String {
            // ...
        }
    }
    
    pub mod websocket {
        pub fn connect(url: &str) {
            // ...
        }
    }
}

// Usage:
use my_lib::network::http;
http::get("https://example.com");

// Or:
use my_lib::network::http::get;
get("https://example.com");
```


### Re-exports

```rust
// src/error.rs
pub use thiserror::Error;
//  ^^^
// Re-export Error trait so users don't need to import thiserror

// Now users can:
use my_crate::error::Error;  // ✅ Works

// Instead of:
use thiserror::Error;  // Don't need to know about thiserror
```


### Absolute vs Relative Paths

```rust
// Absolute path (from crate root)
use crate::error::AppError;
//  ^^^^^
// Always starts from crate root

// Relative path
use super::error::AppError;
//  ^^^^^
// Parent module

use self::submodule::Item;
//  ^^^^
// Current module
```


### Glob Imports (Use Sparingly)

```rust
// Import everything (not recommended)
use crate::error::*;  // Imports all public items

// Better: Import specific items
use crate::error::{AppError, Result};
```

**Why This Matters:**

- Clear code organization
- Encapsulation (hide implementation details)
- Prevents naming conflicts
- Controlled public API

***

## 15. Advanced Patterns

### Pattern 1: Lazy Static with once_cell

```rust
// src/config.rs
use once_cell::sync::Lazy;

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
//                 ^^^^^^^^^^^^
// Initialized once, on first access, thread-safe
    Config::load().unwrap_or_else(|_| {
        eprintln!("Failed to load config, using defaults");
        Config::default()
    })
});

// Usage anywhere in program:
fn some_function() {
    println!("Timeout: {}", CONFIG.http.timeout_secs);
    // First access initializes, subsequent accesses are cheap
}
```

**How It Works:**

```
First access:
Thread 1: CONFIG.http.timeout_secs
         ↓
    Lazy detects uninitialized
         ↓
    Calls Config::load()
         ↓
    Stores result
         ↓
    Returns reference

Subsequent accesses:
Thread 2: CONFIG.http.timeout_secs
         ↓
    Lazy detects initialized
         ↓
    Returns reference immediately (fast!)
```


### Pattern 2: Builder Pattern

```rust
// reqwest client building
let client = Client::builder()
    .timeout(CONFIG.http_timeout())
    .pool_max_idle_per_host(CONFIG.http.pool_max_idle_per_host)
    .user_agent("article-aggregator/0.1.0")
    .build()?;

// Each method returns the builder for chaining
impl ClientBuilder {
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self  // Return self for chaining
    }
    
    pub fn user_agent(mut self, agent: &str) -> Self {
        self.user_agent = Some(agent.to_string());
        self
    }
    
    pub fn build(self) -> Result<Client> {
        Ok(Client { /* ... */ })
    }
}
```


### Pattern 3: Type Aliases for Clarity

```rust
// src/error.rs
pub type Result<T> = std::result::Result<T, AppError>;

// src/rate_limiter.rs
type DirectLimiter = GovernorLimiter<
    governor::state::direct::NotKeyed,
    governor::state::InMemoryState,
    governor::clock::DefaultClock,
>;

// Makes code much more readable!
```


### Pattern 4: Error Context with map_err

```rust
// src/config.rs
let config = ConfigBuilder::builder()
    .build()
    .map_err(|e| AppError::ConfigError(e.to_string()))?;
    //^^^^^^^
    // Transform one error type to another

let cfg: Self = config
    .try_deserialize()
    .map_err(|e| AppError::ConfigError(e.to_string()))?;
```


### Pattern 5: Option Combinators

```rust
// src/model.rs
pub fn searchable_text(&self) -> String {
    match &self.description {
        Some(desc) => format!("{} {}", self.title, desc),
        None => self.title.clone(),
    }
}

// Alternative with combinator:
pub fn searchable_text(&self) -> String {
    self.description
        .as_ref()
        .map(|desc| format!("{} {}", self.title, desc))
        .unwrap_or_else(|| self.title.clone())
}
```


### Pattern 6: Newtype Pattern

```rust
// Wrap primitive types for type safety
pub struct ArticleId(u64);
pub struct UserId(u64);

// Now can't accidentally mix them:
fn get_article(id: ArticleId) -> Article { /* ... */ }
fn get_user(id: UserId) -> User { /* ... */ }

// get_article(UserId(123));  // ❌ Compile error!
```


### Pattern 7: Interior Mutability (Not Used in Code, But Important)

```rust
use std::cell::RefCell;
use std::rc::Rc;

// RefCell allows mutation through shared reference
let data = Rc::new(RefCell::new(vec![1, 2, 3]));

let data_clone = Rc::clone(&data);
data_clone.borrow_mut().push(4);  // Mutable borrow checked at runtime

println!("{:?}", data.borrow());  // [1, 2, 3, 4]
```


### Pattern 8: From/Into for Conversions

```rust
// src/error.rs
impl AppError {
    pub fn http_error(url: impl Into<String>, err: impl Display) -> Self {
    //                     ^^^^^^^^^^^^^^^^
    // Accepts anything that can convert to String
        Self::HttpError {
            url: url.into(),  // Converts to String
            message: err.to_string(),
        }
    }
}

// Can call with &str or String:
AppError::http_error("https://...", error);  // &str → String
AppError::http_error(url_string, error);     // String → String
```


### Pattern 9: Iterator Adapters

```rust
// src/analyzer.rs
let matched: Vec<String> = keyword_counts
    .iter()
    .enumerate()                    // Add indices
    .filter(|(_, (count, _))| *count > 0)  // Filter non-zero
    .map(|(idx, _)| keywords[idx].clone()) // Extract keywords
    .collect();                     // Collect to Vec

// Lazy evaluation - nothing computed until .collect()
```


### Pattern 10: Async Stream Processing

```rust
// src/fetcher.rs
use futures::stream::{self, StreamExt};

stream::iter(futures)              // Create stream of futures
    .buffer_unordered(10)          // Execute 10 concurrently
    .collect::<Vec<_>>()           // Collect results
    .await
```


***

## Summary: Key Takeaways for Engineering Students

### Memory Safety

- **Ownership**: Every value has one owner; no garbage collection needed
- **Borrowing**: Temporary access without ownership transfer
- **Lifetimes**: Compiler ensures references are always valid
- **Result**: Zero runtime overhead, all checked at compile time


### Concurrency

- **Arc**: Share data across threads safely
- **Atomics**: Lock-free synchronization primitives
- **Send/Sync**: Compiler-enforced thread safety
- **Rayon**: Data parallelism for CPU-bound work
- **Tokio**: Async/await for I/O-bound work


### Type System

- **Generics**: Write code once, use with many types
- **Traits**: Shared behavior across types
- **Enums**: Type-safe variants with pattern matching
- **Result/Option**: Explicit error handling and nullable types


### Ecosystem

- **Serde**: Universal serialization framework
- **thiserror**: Declarative error definitions
- **Tokio**: Production-ready async runtime
- **Rayon**: Automatic parallelization


### Best Practices

- Encapsulate with private fields, public methods
- Use type aliases for complex types
- Leverage derive macros to reduce boilerplate
- Prefer ? operator for error propagation
- Use Arc for shared data, spawn_blocking for CPU work
- Configure via files and environment variables

This project demonstrates how Rust's features combine to create safe, fast, concurrent systems with minimal runtime overhead and maximum compile-time guarantees.
<span style="display:none">[^1][^2][^3][^4][^5][^6][^7][^8][^9]</span>

<div align="center">⁂</div>

[^1]: https://doc.rust-lang.org/rust-by-example/

[^2]: https://rust-lang.org/learn/

[^3]: https://www.w3schools.com/rust/

[^4]: https://www.upskillcampus.com/blog/rust-programming-language-tutorial/

[^5]: https://google.github.io/comprehensive-rust/

[^6]: https://www.youtube.com/watch?v=BpPEoZW5IiY

[^7]: https://www.reddit.com/r/rust/comments/15b9rl5/rust_tutorial_that_actually_teaches_rust/

[^8]: https://www.geeksforgeeks.org/rust/introduction-to-rust-programming-language/

[^9]: https://blog.jetbrains.com/rust/2024/09/20/how-to-learn-rust/

