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

pub fn init_rayon_pool(num_threads: usize) {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .expect("Failed to initialize Rayon thread pool");
}

pub fn score_articles(articles: Vec<Article>, keywords: &[String]) -> Result<Vec<ScoredArticle>> {
    if keywords.is_empty() {
        return Err(AppError::AnalyzerError("no keywords configured".into()));
    }

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
    let text = article.searchable_text().to_lowercase();

    let mut keyword_counts: Vec<usize> = vec![0; keywords.len()];

    for mat in ac.find_iter(&text) {
        keyword_counts[mat.pattern().as_usize()] += 1;
    }

    let mut matched_keywords = Vec::new();
    let mut total_score = 0.0;

    for (idx, &count) in keyword_counts.iter().enumerate() {
        if count > 0 {
            matched_keywords.push(keywords[idx].clone());
            // Logarithmic scoring to prevent single keyword dominance
            total_score += 1.0 + (count as f64).ln();
        }
    }

    // Ensure score is finite
    let final_score = if total_score.is_finite() {
        total_score
    } else {
        0.0
    };

    (final_score, matched_keywords)
}
