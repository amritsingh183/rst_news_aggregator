use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Article {
    title: String,
    url: String,
    source: String,
    description: Option<String>,
}

impl Article {
    pub fn new(title: String, url: String, source: String) -> Self {
        Self {
            title,
            url,
            source,
            description: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn searchable_text(&self) -> Cow<'_, str> {
        match &self.description {
            Some(desc) => Cow::Owned(format!("{} {}", self.title, desc)),
            None => Cow::Borrowed(&self.title),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct HackerNewsItem {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
}
