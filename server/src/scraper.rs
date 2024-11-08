use crate::error::SearchError;
use async_trait::async_trait;
use rand::seq::SliceRandom;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct Breadcrumb {
    pub text: String,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub snippet: String,
    pub source: String,
    pub score: f64,
    pub engine: String,
    pub favicon_url: Option<String>,
    pub site_name: Option<String>,
    pub breadcrumbs: Vec<Breadcrumb>,
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

    fn extract_favicon(&self, div: &scraper::ElementRef) -> Option<String> {
        let favicon_selector = Selector::parse("img.XNo5Ab").unwrap();
        div.select(&favicon_selector)
            .next()
            .and_then(|img| img.value().attr("src").map(String::from))
            .or_else(|| {
                // Fallback: construire l'URL du favicon à partir du domaine
                let link_selector = Selector::parse("a").unwrap();
                div.select(&link_selector)
                    .next()
                    .and_then(|a| a.value().attr("href"))
                    .and_then(|url| url::Url::parse(url).ok())
                    .map(|url| {
                        format!(
                            "https://www.google.com/s2/favicons?domain={}",
                            url.host_str().unwrap_or_default()
                        )
                    })
            })
    }

    fn extract_site_info(&self, div: &scraper::ElementRef) -> (Option<String>, Vec<Breadcrumb>) {
        let cite_selector = Selector::parse("cite.qLRx3b").unwrap();
        let breadcrumbs_selector = Selector::parse("span.VuuXrf").unwrap();

        let site_element = div.select(&cite_selector).next();

        let site_name = site_element
            .and_then(|cite| cite.select(&breadcrumbs_selector).next())
            .map(|span| span.text().collect::<String>());

        let breadcrumbs = site_element
            .map(|cite| {
                cite.text()
                    .collect::<String>()
                    .split('›')
                    .map(|part| Breadcrumb {
                        text: part.trim().to_string(),
                        url: None, // Google ne fournit pas les URLs individuelles des breadcrumbs
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![]);

        (site_name, breadcrumbs)
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

                let favicon_url = self.extract_favicon(&div);
                let (site_name, breadcrumbs) = self.extract_site_info(&div);

                Some(SearchResult {
                    title,
                    link,
                    snippet,
                    source: self.name().to_string(),
                    score: 0.0,
                    engine: self.name().to_string(),
                    favicon_url,
                    site_name,
                    breadcrumbs,
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

    fn extract_favicon(&self, result: &scraper::ElementRef) -> Option<String> {
        let url = result
            .select(&Selector::parse(".result__url").unwrap())
            .next()
            .map(|url| url.text().collect::<String>())?;

        // DuckDuckGo n'affiche pas directement les favicons, on utilise donc un service tiers
        Some(format!("https://www.google.com/s2/favicons?domain={}", url))
    }

    fn extract_site_info(&self, result: &scraper::ElementRef) -> (Option<String>, Vec<Breadcrumb>) {
        let url_selector = Selector::parse(".result__url").unwrap();

        let url_text = result
            .select(&url_selector)
            .next()
            .map(|url| url.text().collect::<String>());

        let site_name = url_text
            .clone()
            .map(|url| url.split('/').next().unwrap_or_default().to_string());

        let breadcrumbs = url_text
            .map(|url| {
                url.split('/')
                    .filter(|part| !part.is_empty())
                    .map(|part| Breadcrumb {
                        text: part.to_string(),
                        url: None,
                    })
                    .collect()
            })
            .unwrap();

        (site_name, breadcrumbs)
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

                let favicon_url = self.extract_favicon(&result);
                let (site_name, breadcrumbs) = self.extract_site_info(&result);

                Some(SearchResult {
                    title,
                    link: format!(
                        "https://{}",
                        link.trim_start_matches(|c: char| !c.is_alphanumeric())
                    ),
                    snippet,
                    source: self.name().to_string(),
                    score: 0.0,
                    engine: self.name().to_string(),
                    favicon_url,
                    site_name,
                    breadcrumbs,
                })
            })
            .collect()
    }
}
