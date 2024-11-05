use crate::error::SearchError;
use async_trait::async_trait;
use rand::seq::SliceRandom;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub snippet: String,
    pub source: String,
    pub score: f64,
    pub engine: String,
}

const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
];

#[async_trait]
pub trait SearchEngine: Send + Sync {
    fn name(&self) -> &'static str;
    fn base_url(&self) -> &'static str;

    async fn search(
        &self,
        query: &str,
        page: u32,
        date_range: Option<&str>,
        region: Option<&str>,
        language: Option<&str>,
    ) -> Result<Vec<SearchResult>, SearchError>;

    async fn fetch_html(&self, url: &str) -> Result<String, SearchError> {
        let client = reqwest::Client::builder()
            .user_agent(*USER_AGENTS.choose(&mut rand::thread_rng()).unwrap())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(client
            .get(url)
            .header("Accept", "text/html")
            .header("Accept-Language", "fr-FR,fr;q=0.9")
            .send()
            .await?
            .text()
            .await?)
    }

    fn parse_results(&self, html: &str) -> Vec<SearchResult>;
}

pub struct GoogleScraper;

impl GoogleScraper {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SearchEngine for GoogleScraper {
    fn name(&self) -> &'static str {
        "Google"
    }

    fn base_url(&self) -> &'static str {
        "https://www.google.com/search"
    }

    async fn search(
        &self,
        query: &str,
        page: u32,
        date_range: Option<&str>,
        region: Option<&str>,
        language: Option<&str>,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let start = if page > 1 { (page - 1) * 10 } else { 0 };
        let url = format!(
            "{}?q={}&start={}&num=10&hl=fr",
            self.base_url(),
            query,
            start
        );

        let html = self.fetch_html(&url).await?;
        Ok(self.parse_results(&html))
    }

    fn parse_results(&self, html: &str) -> Vec<SearchResult> {
        let document = Html::parse_document(html);
        let div_selector = Selector::parse("div.g").unwrap();
        let title_selector = Selector::parse("h3").unwrap();
        let link_selector = Selector::parse("a").unwrap();
        let snippet_selector = Selector::parse("div.VwiC3b").unwrap();

        document
            .select(&div_selector)
            .filter_map(|div| {
                let title = div
                    .select(&title_selector)
                    .next()?
                    .text()
                    .collect::<String>();

                let link = div
                    .select(&link_selector)
                    .next()?
                    .value()
                    .attr("href")?
                    .to_string();

                if !link.starts_with("http") {
                    return None;
                }

                let snippet = div
                    .select(&snippet_selector)
                    .next()
                    .map(|s| s.text().collect::<String>())
                    .unwrap_or_default();

                Some(SearchResult {
                    title,
                    link,
                    snippet,
                    source: self.name().to_string(),
                    score: 0.0,
                    engine: self.name().to_string(),
                })
            })
            .collect()
    }
}

pub struct DuckDuckGoScraper;

impl DuckDuckGoScraper {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SearchEngine for DuckDuckGoScraper {
    fn name(&self) -> &'static str {
        "DuckDuckGo"
    }

    fn base_url(&self) -> &'static str {
        "https://html.duckduckgo.com/html"
    }

    async fn search(
        &self,
        query: &str,
        page: u32,
        date_range: Option<&str>,
        region: Option<&str>,
        language: Option<&str>,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let url = if page == 1 {
            format!("{}?q={}", self.base_url(), query)
        } else {
            format!("{}?q={}&s={}", self.base_url(), query, (page - 1) * 10)
        };

        let html = self.fetch_html(&url).await?;
        Ok(self.parse_results(&html))
    }

    fn parse_results(&self, html: &str) -> Vec<SearchResult> {
        let document = Html::parse_document(html);
        let result_selector = Selector::parse(".result").unwrap();
        let title_selector = Selector::parse(".result__title").unwrap();
        let link_selector = Selector::parse(".result__url").unwrap();
        let snippet_selector = Selector::parse(".result__snippet").unwrap();

        document
            .select(&result_selector)
            .filter_map(|result| {
                let title = result
                    .select(&title_selector)
                    .next()?
                    .text()
                    .collect::<String>();

                let link = result
                    .select(&link_selector)
                    .next()?
                    .text()
                    .collect::<String>();

                let snippet = result
                    .select(&snippet_selector)
                    .next()
                    .map(|s| s.text().collect::<String>())
                    .unwrap_or_default();

                Some(SearchResult {
                    title,
                    link,
                    snippet,
                    source: self.name().to_string(),
                    score: 0.0,
                    engine: self.name().to_string(),
                })
            })
            .collect()
    }
}
