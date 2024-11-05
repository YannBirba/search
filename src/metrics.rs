use metrics::{counter, gauge, histogram};
use std::time::{Duration, Instant};

pub struct SearchMetrics;

impl SearchMetrics {
    // Record timing for a search operation
    pub fn record_search_time(engine: &str, duration: Duration) {
        // histogram!("search_duration_seconds", duration.as_secs_f64(), "engine" => engine.to_string());
    }

    // Record success/failure of search operations
    pub fn record_search_result(engine: &str, success: bool) {
        counter!("search_total", "engine" => engine.to_string(), "success" => success.to_string());
    }

    // Record number of results returned
    pub fn record_results_count(engine: &str, count: u64) {
        // gauge!("search_results_count", count as f64, "engine" => engine.to_string());
    }

    // Record cache operations
    pub fn record_cache_hit() {
        counter!("cache_hits_total");
    }

    pub fn record_cache_miss() {
        counter!("cache_misses_total");
    }
}