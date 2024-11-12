use crate::error::SearchError;
use async_trait::async_trait;
use rand::seq::SliceRandom;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use serde_json::Value;

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
    pub favicon_url: Option<String>,
    pub site_name: Option<String>,
    pub breadcrumbs: Vec<Breadcrumb>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct QuickAnswer {
    pub answer_type: String,
    #[serde(flatten)]
    pub data: Value,
    pub source: String,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct Definition {
    pub term: String,
    pub definition: String,
}

impl QuickAnswer {
    pub fn new_definition(term: String, definition: String, source: Option<String>) -> Self {
        Self {
            answer_type: "definition".to_string(),
            data: serde_json::to_value(Definition {
                term,
                definition,
            }).unwrap(),
            source: source.unwrap_or_default(),
        }
    }
}

impl PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for SearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for SearchResult {}

impl Ord for SearchResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .score
            .partial_cmp(&self.score)
            .unwrap()
            .then_with(|| self.title.cmp(&other.title))
            .then_with(|| self.link.cmp(&other.link))
    }

    fn max(self, other: Self) -> Self
    where
        Self: Sized,
    {
        std::cmp::max_by(self, other, Ord::cmp)
    }

    fn min(self, other: Self) -> Self
    where
        Self: Sized,
    {
        std::cmp::min_by(self, other, Ord::cmp)
    }

    fn clamp(self, min: Self, max: Self) -> Self
    where
        Self: Sized,
        Self: PartialOrd,
    {
        assert!(min <= max);
        if self < min {
            min
        } else if self > max {
            max
        } else {
            self
        }
    }
}

struct SearchResultWrapper(SearchResult);

impl PartialOrd for SearchResultWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SearchResultWrapper {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.score.partial_cmp(&other.0.score).unwrap()
    }
}

impl PartialEq for SearchResultWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.score == other.0.score
    }
}

impl Eq for SearchResultWrapper {}

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

    async fn quick_answer(&self, query: &str) -> Result<Option<QuickAnswer>, SearchError> {
        Ok(None)
    }
}

pub struct GoogleScraper {
    client: reqwest::Client,
}

impl GoogleScraper {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(*USER_AGENTS.choose(&mut rand::thread_rng()).unwrap())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        Self { client }
    }

    async fn fetch_html(&self, url: &str) -> Result<String, SearchError> {
        Ok(self
            .client
            .get(url)
            .header("Accept", "text/html")
            .header("Accept-Language", "fr-FR,fr;q=0.9")
            .send()
            .await?
            .text()
            .await?)
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
        let site_name_selector = Selector::parse("span.VuuXrf").unwrap();
        let breadcrumbs_selector = Selector::parse("cite.qLRx3b").unwrap();

        let site_name = div
            .select(&site_name_selector)
            .next()
            .map(|span| span.text().collect::<String>());

        let breadcrumbs = div
            .select(&breadcrumbs_selector)
            .next()
            .map(|cite| {
                let mut url_accumulator = String::new();
                cite.text()
                    .collect::<String>()
                    .split('›')
                    .map(|part| {
                        url_accumulator.push_str(part.trim());
                        let breadcrumb = Breadcrumb {
                            text: part.trim().to_string(),
                            url: Some(url_accumulator.clone()),
                        };
                        url_accumulator.push('/');
                        breadcrumb
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![]);

        (site_name, breadcrumbs)
    }

    async fn extract_quick_answer(&self, query: &str) -> Result<Option<QuickAnswer>, SearchError> {
        let url = format!("{}?q={}", self.base_url(), query);
        let html = self.fetch_html(&url).await?;
        let document = Html::parse_document(&html);

        let definition_selector = Selector::parse("div.TzHB6b.j8lBAb.p7kDMc.cLjAic.LMRCfc").unwrap();
        let term_selector = Selector::parse("div.RES9jf.xWMiCc.JgzqYd span").unwrap();

        if let (Some(definition), Some(term)) = (
            document.select(&definition_selector).next(),
            document.select(&term_selector).next(),
        ) {
            return Ok(Some(QuickAnswer::new_definition(
                term.text().collect::<String>().trim().to_string(),
                definition.text().collect::<String>().trim().to_string(),
                Some("Google".to_string()),
            )));
        }

        Ok(None)
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
                    favicon_url,
                    site_name,
                    breadcrumbs,
                })
            })
            .collect()
    }

    async fn quick_answer(&self, query: &str) -> Result<Option<QuickAnswer>, SearchError> {
        self.extract_quick_answer(query).await
    }
}

pub struct DuckDuckGoScraper {
    client: reqwest::Client,
}

impl DuckDuckGoScraper {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(*USER_AGENTS.choose(&mut rand::thread_rng()).unwrap())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        Self { client }
    }

    async fn fetch_html(&self, url: &str) -> Result<String, SearchError> {
        Ok(self
            .client
            .get(url)
            .header("Accept", "text/html")
            .header("Accept-Language", "fr-FR,fr;q=0.9")
            .send()
            .await?
            .text()
            .await?)
    }
    fn extract_favicon(&self, result: &scraper::ElementRef) -> Option<String> {
        let url = result
            .select(&Selector::parse(".result__url").unwrap())
            .next()
            .map(|url| url.text().collect::<String>().trim().to_string())?;

        // DuckDuckGo n'affiche pas directement les favicons, on utilise donc un service tiers
        Some(format!("https://www.google.com/s2/favicons?domain={}", url))
    }

    fn extract_site_info(&self, result: &scraper::ElementRef) -> Vec<Breadcrumb> {
        let url_selector = Selector::parse(".result__url").unwrap();

        let url_text = result
            .select(&url_selector)
            .next()
            .map(|url| url.text().collect::<String>());

        let breadcrumbs = url_text
            .map(|url| {
                let mut url_accumulator = String::new();
                url.split('/')
                    .filter(|part| !part.is_empty())
                    .map(|part| {
                        if !url_accumulator.ends_with('/') && !url_accumulator.is_empty() {
                            url_accumulator.push('/');
                        }
                        url_accumulator.push_str(part.trim());
                        Breadcrumb {
                            text: part.to_string().trim().to_string(),
                            url: Some(url_accumulator.clone()),
                        }
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![]);

        breadcrumbs
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
                let breadcrumbs = self.extract_site_info(&result);

                Some(SearchResult {
                    title: title.trim().to_string(),
                    link: format!(
                        "https://{}",
                        link.trim_start_matches(|c: char| !c.is_alphanumeric())
                    ),
                    snippet: snippet.trim().to_string(),
                    source: self.name().to_string(),
                    score: 0.0,
                    favicon_url,
                    site_name: None,
                    breadcrumbs,
                })
            })
            .collect()
    }
}