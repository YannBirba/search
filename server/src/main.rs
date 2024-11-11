use axum::extract::rejection::JsonRejection;
use axum::extract::FromRequest;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{routing::get, Router};
use futures::stream::{FuturesUnordered, StreamExt};
use search::cache::{Cache, RedisCache};
use search::metrics::SearchMetrics;
use search::rate_limiter::RateLimiter;
use search::scoring::ResultScorer;
use search::scraper::SearchResult;
use search::scraper::{DuckDuckGoScraper, GoogleScraper, SearchEngine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BinaryHeap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

struct SearchService {
    engines: Vec<Box<dyn SearchEngine>>,
    cache: Arc<RedisCache>,
    rate_limiter: Arc<RateLimiter>,
}

#[derive(Clone)]
struct AppState {
    search_service: Arc<SearchService>,
}

#[derive(Deserialize)]
struct AutocompleteParams {
    query: String,
}

// Query parameters for search API
#[derive(Deserialize)]
struct SearchParams {
    query: String,
    page: Option<u32>,
    date_range: Option<String>,
    region: Option<String>,
    language: Option<String>,
}

impl SearchService {
    pub fn new(cache: RedisCache) -> Self {
        Self {
            engines: vec![
                Box::new(GoogleScraper::new()),
                Box::new(DuckDuckGoScraper::new()),
            ],
            cache: Arc::new(cache),
            rate_limiter: Arc::new(RateLimiter::new()),
        }
    }

    // Main search function that orchestrates the entire search process
    pub async fn search(
        &self,
        query: &str,
        page: Option<u32>,
        date_range: Option<&str>,
        region: Option<&str>,
        language: Option<&str>,
    ) -> Vec<SearchResult> {
        // Cache key includes new parameters to avoid returning incorrect results
        let cache_key = format!(
            "search:{}:{}:{:?}:{:?}:{:?}",
            query,
            page.unwrap_or(1),
            date_range,
            region,
            language
        );

        // Check cache first
        if let Some(cached_results) = self.cache.get(&cache_key).await {
            SearchMetrics::record_cache_hit();
            return cached_results;
        }

        SearchMetrics::record_cache_miss();

        let mut futures = FuturesUnordered::new();
        for engine in &self.engines {
            let query = query.to_string();
            let page = page.clone();
            let date_range = date_range.map(|s| s.to_string());
            let region = region.map(|s| s.to_string());
            let language = language.map(|s| s.to_string());
            let rate_limiter = &self.rate_limiter;

            futures.push(async move {
                // Check rate limit
                if !rate_limiter.check_rate_limit(engine.name()).await {
                    return Vec::new();
                }

                // Perform search with additional parameters if supported
                match engine
                    .search(
                        &query,
                        page.unwrap_or(1),
                        date_range.as_deref(),
                        region.as_deref(),
                        language.as_deref(),
                    )
                    .await
                {
                    Ok(results) => {
                        SearchMetrics::record_search_result(engine.name(), true);
                        results
                    }
                    Err(_) => {
                        SearchMetrics::record_search_result(engine.name(), false);
                        Vec::new()
                    }
                }
            });
        }

        let mut all_results = Vec::new();
        while let Some(results) = futures.next().await {
            all_results.extend(results);
        }

        // Score and sort results
        for result in &mut all_results {
            result.score = ResultScorer::score_result(result, query);
        }

        // Use a BinaryHeap to sort results by score
        let mut heap = BinaryHeap::new();
        for result in all_results {
            heap.push(result);
        }

        let mut final_results: Vec<_> = heap.into_sorted_vec();

        // Remove duplicates
        final_results = ResultScorer::remove_duplicates(final_results);

        // Cache results
        let _ = self
            .cache
            .set(&cache_key, &final_results, Duration::from_secs(300))
            .await;

        final_results
    }

    pub async fn autocomplete(&self, query: &str) -> Vec<String> {
        let cache_key = format!("autocomplete:{}", query);

        if let Some(cached_results) = self.cache.get(&cache_key).await {
            return cached_results;
        }

        let url = format!(
            "https://www.google.com/complete/search?q={}&cp=4&client=gws-wiz-serp&xssi=t&gs_pcrt=undefined&hl=fr&authuser=0&pq=google%20autocomplete%20search&psi=PT4yZ_aPFZmSkdUP8KizgQo.1731345982335&dpr=1&newwindow=1",
            query
        );

        let response = match reqwest::get(&url).await {
            Ok(resp) => resp,
            Err(err) => {
                eprintln!("Request error: {:?}", err);
                return Vec::new();
            }
        };

        let body = match response.text().await {
            Ok(text) => text,
            Err(err) => {
                eprintln!("Response body error: {:?}", err);
                return Vec::new();
            }
        };

        let mut results = Vec::new();

        for line in body.lines() {
            if line.starts_with("[") {
                if let Ok(json) = serde_json::from_str::<Value>(line) {
                    if let Some(suggestions) = json.get(0).and_then(|v| v.as_array()) {
                        for suggestion in suggestions {
                            if let Some(suggestion_text) =
                                suggestion.get(0).and_then(|s| s.as_str())
                            {
                                results.push(suggestion_text.to_string());
                            }
                        }
                    }
                }
            }
        }

        let _ = self
            .cache
            .set(&cache_key, &results, Duration::from_secs(300))
            .await;

        results
    }
}

// Rename the handler function to avoid conflict with the `search` crate or module.
async fn handle_search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> AppJson<Vec<SearchResult>> {
    let search_service = state.search_service.clone();

    AppJson(
        search_service
            .search(
                &params.query,
                params.page,
                params.date_range.as_deref(),
                params.region.as_deref(),
                params.language.as_deref(),
            )
            .await,
    )
}

async fn handle_autocomplete(
    State(state): State<AppState>,
    Query(params): Query<AutocompleteParams>,
) -> AppJson<Vec<String>> {
    let search_service = state.search_service.clone();

    AppJson(search_service.autocomplete(&params.query).await)
}

#[tokio::main]
async fn main() {
    // Initialize Redis cache
    dotenv::dotenv().ok();
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");

    let cache = match RedisCache::new(redis_url.as_str()).await {
        Ok(cache) => cache,
        Err(e) => {
            eprintln!("Failed to connect to Redis: {}", e);
            return;
        }
    };

    // Clear cache
    cache.flush().await.unwrap();

    // Initialize SearchService and wrap it in AppState
    let search_service = Arc::new(SearchService::new(cache));
    let app_state = AppState { search_service };

    let router = Router::new()
        .route("/api/search", get(handle_search))
        .route("/api/autocomplete", get(handle_autocomplete))
        .layer(CorsLayer::permissive())
        .fallback_service(ServeDir::new("dist"));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, router.with_state(app_state).into_make_service())
        .await
        .unwrap();

    println!("Server running on http://localhost:3000");
}

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
struct AppJson<T>(T);

impl<T> IntoResponse for AppJson<T>
where
    axum::Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

// The kinds of errors we can hit in our application.
enum AppError {
    // The request body contained invalid JSON
    JsonRejection(JsonRejection),
    // Some error from a third party library we're using
    TimeError(time_library::Error),
}

// Tell axum how `AppError` should be converted into a response.
//
// This is also a convenient place to log errors.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // How we want errors responses to be serialized
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let (status, message) = match self {
            AppError::JsonRejection(rejection) => {
                // This error is caused by bad user input so don't log it
                (rejection.status(), rejection.body_text())
            }
            AppError::TimeError(err) => {
                // Because `TraceLayer` wraps each request in a span that contains the request
                // method, uri, etc we don't need to include those details here
                tracing::error!(%err, "error from time_library");

                // Don't expose any details about the error to the client
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_owned(),
                )
            }
        };

        (status, AppJson(ErrorResponse { message })).into_response()
    }
}

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        Self::JsonRejection(rejection)
    }
}

impl From<time_library::Error> for AppError {
    fn from(error: time_library::Error) -> Self {
        Self::TimeError(error)
    }
}

// Imagine this is some third party library that we're using. It sometimes returns errors which we
// want to log.
mod time_library {
    use serde::Serialize;

    #[derive(Serialize, Clone)]
    pub struct Timestamp(u64);

    #[derive(Debug)]
    pub enum Error {
        FailedToGetTime,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "failed to get time")
        }
    }
}
