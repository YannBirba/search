use std::collections::HashSet;

use crate::scraper::SearchResult;
use itertools::Itertools;
use strsim::normalized_levenshtein;

pub struct ResultScorer;

impl ResultScorer {
    // Calculate relevance score for a search result
    pub fn score_result(result: &SearchResult, query: &str) -> f64 {
        let mut score = 0.0;

        // Score based on title relevance
        score += Self::calculate_text_relevance(&result.title, query) * 0.4;

        // Score based on snippet relevance
        score += Self::calculate_text_relevance(&result.snippet, query) * 0.3;

        // Bonus for HTTPS
        if result.link.starts_with("https") {
            score += 0.1;
        }

        // Length penalty for very short snippets
        if result.snippet.len() < 50 {
            score *= 0.8;
        }

        score
    }

    // Calculate text relevance using various metrics
    fn calculate_text_relevance(text: &str, query: &str) -> f64 {
        let text_lower = text.to_lowercase();
        let query_lower = query.to_lowercase();

        // Levenshtein distance for fuzzy matching
        let levenshtein_score = normalized_levenshtein(&text_lower, &query_lower);

        // Exact match bonus
        let contains_exact = text_lower.contains(&query_lower) as i32 as f64;

        // Word match ratio
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let matching_words = query_words
            .iter()
            .filter(|word| text_lower.contains(*word))
            .count() as f64;
        let word_ratio = matching_words / query_words.len() as f64;

        // Combine scores with weights
        0.3 * levenshtein_score + 0.4 * contains_exact + 0.3 * word_ratio
    }

    // Remove duplicate results based on URL similarity
    pub fn remove_duplicates(results: Vec<SearchResult>) -> Vec<SearchResult> {
        let mut seen = HashSet::new();

        results
            .into_iter()
            .filter(|result| {
                let is_duplicate = seen
                    .iter()
                    .any(|&ref seen_result: &String| Self::is_duplicate(&result.link, seen_result));

                if !is_duplicate {
                    seen.insert(result.link.clone());
                }

                !is_duplicate
            })
            .collect()
    }

    // Check if two URLs point to the same content
    fn is_duplicate(url1: &str, url2: &str) -> bool {
        let normalize_url =
            |url: &str| url.trim_end_matches('/').replace("www.", "").to_lowercase();

        let url1_norm = normalize_url(url1);
        let url2_norm = normalize_url(url2);

        url1_norm == url2_norm || normalized_levenshtein(&url1_norm, &url2_norm) > 0.9
    }
}
