use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::core::agent::extract_outline;
use crate::core::io::read_markdown_file_safe;
use crate::tui::app::flatten_headings;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchHit {
    pub file_path: String,
    pub heading_path: String,
    pub line_number: usize,
    pub line_snippet: String,
    pub section_tokens: usize,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultiDocSearchResult {
    pub query: String,
    pub total_hits: usize,
    pub files_searched: usize,
    pub hits: Vec<SearchHit>,
}

type RawHitTuple = (
    SearchHit,
    String, // line_lower
    u32,    // heading_level
    String, // heading_title
    usize,  // line_words
);

use crate::core::query::parse_query;

/// Hyper-parallel search across multiple Markdown files/directories using Rayon worker threads
pub fn search_documents(paths: &[PathBuf], raw_query: &str) -> MultiDocSearchResult {
    let parsed_query = parse_query(raw_query);
    let query_terms = if parsed_query.positive_terms.is_empty() {
        parsed_query.exact_phrases.clone()
    } else {
        parsed_query.positive_terms.clone()
    };

    if parsed_query.is_empty() {
        return MultiDocSearchResult {
            query: raw_query.to_string(),
            total_hits: 0,
            files_searched: 0,
            hits: Vec::new(),
        };
    }

    let file_results: Vec<(Vec<RawHitTuple>, usize, usize)> = paths
        .par_iter()
        .filter_map(|path| {
            if let Ok(content) = read_markdown_file_safe(path) {
                let file_path_str = path.display().to_string();

                if !parsed_query.matches_filters(&file_path_str, &content) {
                    return None;
                }

                let outline = extract_outline(&content);
                let flat_headings = flatten_headings(&outline.headings);
                let lines: Vec<&str> = content.lines().collect();

                let mut local_hits = Vec::new();
                let mut local_sections = 0;
                let mut local_words = 0;

                for (line_idx, line) in lines.iter().enumerate() {
                    let line_lower = line.to_lowercase();
                    let is_hit = if query_terms.is_empty() {
                        true
                    } else {
                        query_terms.iter().any(|term| line_lower.contains(term))
                    };

                    if is_hit {
                        let line_num = line_idx + 1;

                        let current_heading = flat_headings
                            .iter()
                            .rfind(|h| h.start_line <= line_num && line_num <= h.end_line);

                        let (heading_path, heading_level, heading_title, section_tokens) =
                            match current_heading {
                                Some(h) => (
                                    format!("H{}: {}", h.level, h.title),
                                    h.level,
                                    h.title.to_lowercase(),
                                    h.token_count,
                                ),
                                None => (String::from("Root Document"), 0, String::new(), 0),
                            };

                        let line_words = line.split_whitespace().count().max(1);
                        local_sections += 1;
                        local_words += line_words;

                        local_hits.push((
                            SearchHit {
                                file_path: file_path_str.clone(),
                                heading_path,
                                line_number: line_num,
                                line_snippet: line.trim().to_string(),
                                section_tokens,
                                score: 0.0,
                            },
                            line_lower,
                            heading_level,
                            heading_title,
                            line_words,
                        ));
                    }
                }
                Some((local_hits, local_sections, local_words))
            } else {
                None
            }
        })
        .collect();

    let mut raw_hits = Vec::new();
    let files_searched = file_results.len();
    let mut total_sections = 0;
    let mut total_words = 0;

    for (hits, sections, words) in file_results {
        raw_hits.extend(hits);
        total_sections += sections;
        total_words += words;
    }

    let n_docs = files_searched.max(1) as f64;
    let avgdl = if total_sections > 0 {
        total_words as f64 / total_sections as f64
    } else {
        1.0
    };
    let k1 = 1.2;
    let b = 0.75;

    // Calculate unique document frequency df(t) for each query term across raw hits
    let mut df_map: HashMap<String, usize> = HashMap::new();
    for term in &query_terms {
        let unique_docs: std::collections::HashSet<&String> = raw_hits
            .iter()
            .filter(|(hit, line_lower, _, heading_title, _)| {
                line_lower.contains(term) || heading_title.contains(term)
            })
            .map(|(hit, ..)| &hit.file_path)
            .collect();
        df_map.insert(term.clone(), unique_docs.len());
    }

    let mut hits: Vec<SearchHit> = raw_hits
        .into_iter()
        .map(
            |(mut hit, line_lower, heading_level, heading_title, doc_len)| {
                let mut total_score = 0.0;

                for term in &query_terms {
                    let tf = line_lower.matches(term).count() as f64;
                    if tf > 0.0 {
                        let df = (*df_map.get(term).unwrap_or(&1)).max(1) as f64;
                        let idf = ((n_docs - df + 0.5) / (df + 0.5) + 1.0).ln();
                        let num = tf * (k1 + 1.0);
                        let denom = tf + k1 * (1.0 - b + b * (doc_len as f64 / avgdl));
                        let bm25 = idf * (num / denom);

                        let heading_multiplier = if heading_title.contains(term) {
                            match heading_level {
                                1 => 3.0,
                                2 => 2.5,
                                3 => 2.0,
                                _ => 1.5,
                            }
                        } else {
                            1.0
                        };

                        total_score += bm25 * heading_multiplier;
                    }
                }

                hit.file_path = hit.file_path.replace('\\', "/");
                hit.score = (total_score * 100.0).round() / 100.0;
                hit
            },
        )
        .collect();

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    MultiDocSearchResult {
        query: raw_query.to_string(),
        total_hits: hits.len(),
        files_searched,
        hits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_multi_document_search() {
        let temp_dir = std::env::temp_dir();
        let file1_path = temp_dir.join("pankh_search_1.md");
        let file2_path = temp_dir.join("pankh_search_2.md");

        let mut f1 = File::create(&file1_path).unwrap();
        writeln!(f1, "# Architecture\n\nDatabase setup details.").unwrap();

        let mut f2 = File::create(&file2_path).unwrap();
        writeln!(f2, "# Installation\n\nDatabase migration guide.").unwrap();

        let results = search_documents(&[file1_path.clone(), file2_path.clone()], "Database");
        assert_eq!(results.total_hits, 2);
        assert_eq!(results.files_searched, 2);
        assert!(results
            .hits
            .iter()
            .any(|h| h.heading_path.contains("Architecture")));
        assert!(results
            .hits
            .iter()
            .any(|h| h.heading_path.contains("Installation")));

        let _ = std::fs::remove_file(file1_path);
        let _ = std::fs::remove_file(file2_path);
    }

    #[test]
    fn test_bm25_heading_multiplier_boost() {
        let temp_dir = std::env::temp_dir();
        let file1 = temp_dir.join("bm25_test_h1.md");
        let file2 = temp_dir.join("bm25_test_body.md");

        let mut f1 = File::create(&file1).unwrap();
        writeln!(f1, "# Database Architecture\n\nOverview.").unwrap();

        let mut f2 = File::create(&file2).unwrap();
        writeln!(
            f2,
            "# Section\n\nDatabase migration guide for production database."
        )
        .unwrap();

        let results = search_documents(&[file1.clone(), file2.clone()], "Database");
        assert_eq!(results.total_hits, 2);
        assert!(results.hits[0]
            .heading_path
            .contains("Database Architecture"));
        assert!(results.hits[0].score > results.hits[1].score);

        let _ = std::fs::remove_file(file1);
        let _ = std::fs::remove_file(file2);
    }

    #[test]
    fn test_bm25_dynamic_df_scoring() {
        let temp_dir = std::env::temp_dir();
        let f1 = temp_dir.join("df_test_common.md");
        let f2 = temp_dir.join("df_test_rare.md");

        let mut file1 = File::create(&f1).unwrap();
        writeln!(file1, "# Common Section\n\nWord1 Word2.").unwrap();

        let mut file2 = File::create(&f2).unwrap();
        writeln!(file2, "# Rare Section\n\nWord1 SpecialRareTerm.").unwrap();

        let results = search_documents(&[f1.clone(), f2.clone()], "Word1 SpecialRareTerm");
        assert_eq!(results.total_hits, 2);
        assert!(results.hits[0].line_snippet.contains("SpecialRareTerm"));

        let _ = std::fs::remove_file(f1);
        let _ = std::fs::remove_file(f2);
    }

    #[test]
    fn test_parallel_multi_document_search() {
        let temp_dir = std::env::temp_dir();
        let mut paths = Vec::new();
        for i in 0..10 {
            let p = temp_dir.join(format!("parallel_test_{}.md", i));
            let mut f = File::create(&p).unwrap();
            writeln!(
                f,
                "# Section {}\n\nContent matching query term in file {}.",
                i, i
            )
            .unwrap();
            paths.push(p);
        }

        let results = search_documents(&paths, "query");
        assert_eq!(results.total_hits, 10);
        assert_eq!(results.files_searched, 10);

        for p in paths {
            let _ = std::fs::remove_file(p);
        }
    }
}
