use crate::config::CONFIG;
use crate::error::{AppError, Result};
use crate::model::Article;
use aho_corasick::AhoCorasick;
use rayon::prelude::*;
use std::sync::Arc;

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
        self.relevance_score
    }

    pub fn matched_keywords(&self) -> &[String] {
        &self.matched_keywords
    }
}

/// Initialize a separate Rayon thread pool to avoid blocking Tokio runtime
pub fn init_rayon_pool() {
    // Build global only once in program lifetime
    rayon::ThreadPoolBuilder::new()
        .num_threads(CONFIG.analyzer.rayon_threads)
        .thread_name(|i| format!("rayon-worker-{}", i))
        .build_global()
        .expect("Failed to build Rayon thread pool");
}

/// Score articles using efficient Aho-Corasick algorithm for keyword matching
pub fn score_articles(articles: Vec<Article>, keywords: &[String]) -> Result<Vec<ScoredArticle>> {
    if keywords.is_empty() {
        return Err(AppError::AnalyzerError("no keywords configured".into()));
    }

    // Build Aho-Corasick automaton once for efficient multi-pattern matching
    let patterns: Vec<&str> = keywords.iter().map(|s| s.as_str()).collect();
    let ac = AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .build(&patterns)
        .map_err(|e| AppError::AnalyzerError(format!("failed to build AC: {e}")))?;
    let ac = Arc::new(ac);

    let scored = articles
        .into_par_iter()
        .map(|article| {
            let (score, matched) = calculate_relevance(&article, &ac, keywords);
            ScoredArticle {
                article,
                relevance_score: score,
                matched_keywords: matched,
            }
        })
        .collect();

    Ok(scored)
}

fn calculate_relevance(
    article: &Article,
    ac: &AhoCorasick,
    keywords: &[String],
) -> (f64, Vec<String>) {
    // Lowercasing once to align with ASCII-insensitive matching and stable indices
    let text = article.searchable_text().to_lowercase();

    // (count, reserved) layout keeps option to weight by field later
    let mut keyword_counts: Vec<(usize, usize)> = vec![(0, 0); keywords.len()];
    let mut total_matches = 0;

    for mat in ac.find_iter(&text) {
        let pattern_id = mat.pattern().as_usize();
        if let Some(slot) = keyword_counts.get_mut(pattern_id) {
            slot.0 += 1;
            total_matches += 1;
        }
    }

    let unique_keywords = keyword_counts.iter().filter(|(c, _)| *c > 0).count();
    let score = if total_matches == 0 {
        0.0
    } else {
        (total_matches as f64).powf(1.5) * (unique_keywords as f64).powf(1.2)
    };

    let matched: Vec<String> = keyword_counts
        .iter()
        .enumerate()
        .filter(|(_, (count, _))| *count > 0)
        .map(|(idx, _)| keywords[idx].clone())
        .collect();

    (score, matched)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scoring() {
        let articles = vec![
            Article::new(
                "Rust async performance".to_string(),
                "http://example.com/1".to_string(),
                "test".to_string(),
            ),
            Article::new(
                "Python tutorial".to_string(),
                "http://example.com/2".to_string(),
                "test".to_string(),
            ),
        ];

        let keywords = vec!["rust".to_string(), "async".to_string()];
        let scored = score_articles(articles, &keywords).unwrap();

        assert_eq!(scored.len(), 2);
        assert!(scored[0].relevance_score > scored[1].relevance_score);
    }
}
