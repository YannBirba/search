use std::{collections::HashSet, vec};

use crate::scraper::SearchResult;
use strsim::normalized_levenshtein;
use unidecode::unidecode;
use url::Url;

pub struct ResultScorer;

impl ResultScorer {
    // Calculer le score de pertinence pour un résultat de recherche
    pub fn score_result(result: &SearchResult, query: &str) -> f64 {
        let mut score = 0.0;
        let normalized_query = unidecode(&query.to_lowercase());
        let normalized_title = unidecode(&result.title.to_lowercase());
        let normalized_snippet = unidecode(&result.snippet.to_lowercase());
        let normalized_link = unidecode(&result.link.to_lowercase());

        let urls_blacklist: Vec<&str> = vec![
            "bfmtv.com",
            "60millions-mag.com",
            "bbc.com",
            "jeuxvideo.com",
            "linternaute.fr",
            "lefigaro.fr",
            "leparisien.fr",
            "lequipe.fr",
            "ladepeche.fr",
            "lepoint.fr",
            "lejdd.fr",
            "lesechos.fr",
            "liberation.fr",
            "lci.fr",
            "lemondedutabac.com",
            "16personalities.com",
            "freecodecamp.org",
            "dev.to",
            "medium.com",
            "w3schools.com",
        ];

        let relevant_urls: Vec<&str> = vec![
            "github.com",
            "docs.rs",
            "react.dev",
            "wikipedia.org",
            "stackoverflow.com",
            "youtube.com",
            "reddit.com",
            "wordpress.com",
            "gitlab.com",
            "bitbucket.org",
            "sourceforge.net",
            "crates.io",
            "npmjs.com",
            "rust-lang.org",
            "mozilla.org",
            "developer.mozilla.org",
            "developer.android.com",
            "developer.apple.com",
            "developer.microsoft.com",
            "developer.chrome.com",
            "dictionnaire.lerobert.com",
            "gouv.fr",
            "openclassrooms.com",
            "larousse.fr",
            "cnrtl.fr",
        ];

        let bonus_words: Vec<&str> = vec![
            "definition",
            "meaning",
            "signification",
            "sens",
            "tuto",
            "tutorial",
            "guide",
            "cours",
            "explanation",
            "explication",
            "significations",
            "sens",
            "tutoriel",
            "guides",
            "cours",
            "explications",
            "wikipedia",
            "wiki",
            "dictionnaire",
            "dictionary",
            "docs",
            "documentation",
        ];

        // Score basé sur la pertinence du titre
        score += Self::calculate_text_relevance(&normalized_title, &normalized_query) * 0.5;

        // Score basé sur la pertinence du snippet
        score += Self::calculate_text_relevance(&normalized_snippet, &normalized_query) * 0.3;

        // Score basé sur la pertinence du lien
        score += Self::calculate_text_relevance(&normalized_link, query) * 0.2;

        // Bonus pour HTTPS ou pour wikipedia
        if normalized_link.starts_with("https") {
            score += 0.5;
        }

        // TODO Pénalité pour le contenu ancien
        // if let Some(date) = result.publish_date {
        //     let age_in_days = (chrono::Utc::now().date() - date).num_days();
        //     if age_in_days > 365 {
        //         score *= 0.9; // Réduire le score de 10% si le contenu a plus d'un an
        //     }
        // }

        // Penalty for very short or very long snippets
        if normalized_snippet.len() < 50 || normalized_snippet.len() > 150 {
            score *= 0.8;
        }

        // Penalty for blacklisted URLs
        if urls_blacklist
            .iter()
            .any(|&blacklisted_url| normalized_link.contains(blacklisted_url))
        {
            score *= 0.25;
        }

        // Bonus for relevant URLs
        if relevant_urls
            .iter()
            .any(|&relevant_url| normalized_link.contains(relevant_url))
        {
            score += 0.3;
        }

        // Bonus for exact match in title
        if normalized_title == normalized_query {
            score += 0.5;
        }

        // Bonus for exact match in snippet
        if normalized_snippet == normalized_query {
            score += 0.75;
        }

        // Bonus for choosen words on the title, snippet or link
        if bonus_words.iter().any(|&bonus_word| {
            normalized_title.contains(bonus_word)
                || normalized_snippet.contains(bonus_word)
                || normalized_link.contains(bonus_word)
        }) {
            score += 0.5;
        }

        // limit float to 2 decimal places
        (score * 100.0).round() / 100.0
    }

    // Calculer la pertinence du texte en utilisant le comptage des termes
    fn calculate_text_relevance(text: &str, query: &str) -> f64 {
        // Levenshtein distance for fuzzy matching
        let levenshtein_score = normalized_levenshtein(&text, &query);

        // Exact match bonus
        let contains_exact = text.contains(&query) as i32 as f64;

        // Word match ratio
        let query_words: Vec<&str> = query.split_whitespace().collect();
        let matching_words = query_words
            .iter()
            .filter(|word| text.contains(*word))
            .count() as f64;
        let word_ratio = matching_words / query_words.len() as f64;

        // Combine scores with weights
        0.3 * levenshtein_score + 0.4 * contains_exact + 0.3 * word_ratio
    }

    // Remove duplicate results based on URL similarity
    pub fn remove_duplicates(results: Vec<SearchResult>) -> Vec<SearchResult> {
        let mut seen: Vec<SearchResult> = Vec::new();
        let mut unique_results = Vec::new();

        for result in results {
            let is_duplicate = seen
                .iter()
                .any(|seen_link| Self::is_duplicate(&result, seen_link));

            if !is_duplicate {
                seen.push(result.clone());
                unique_results.push(result);
            }
        }

        unique_results
    }

    // Check if two URLs point to the same content
    fn is_duplicate(result1: &SearchResult, result2: &SearchResult) -> bool {
        let url1 = &result1.link;
        let url2 = &result2.link;
        let normalize_url = |url: &str| {
            if let Ok(parsed_url) = Url::parse(url) {
                let mut normalized = format!(
                    "{}{}",
                    parsed_url.host_str().unwrap_or(""),
                    parsed_url.path()
                );
                normalized = normalized
                    .trim_end_matches('/')
                    .replace("www.", "")
                    .to_lowercase();
                normalized
            } else {
                url.trim_end_matches('/').replace("www.", "").to_lowercase()
            }
        };

        let url1_norm = normalize_url(url1);
        let url2_norm = normalize_url(url2);

        url1_norm == url2_norm
            || normalized_levenshtein(&url1_norm, &url2_norm) > 0.9
            || result1.title == result2.title
            || result1.snippet == result2.snippet
    }
}
