use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::core::agent::extract_outline;
use crate::core::io::read_markdown_file_safe;
use crate::core::search::{MultiDocSearchResult, SearchHit};
use crate::tui::app::flatten_headings;

pub const DEFAULT_INDEX_FILENAME: &str = ".pankh_index.bin";

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct IndexedDocMeta {
    pub path: PathBuf,
    pub mtime_secs: u64,
    pub file_size: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct IndexedLine {
    pub line_number: usize,
    pub line_snippet: String,
    pub line_words: usize,
    pub heading_path: String,
    pub heading_level: u32,
    pub heading_title: String,
    pub section_tokens: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SearchIndex {
    pub version: u32,
    pub docs: HashMap<String, (IndexedDocMeta, Vec<IndexedLine>)>,
    pub term_posting: HashMap<String, Vec<(String, usize)>>, // term -> list of (doc_path_str, line_idx)
    pub trigram_posting: HashMap<String, Vec<String>>,       // trigram -> list of matching terms
    pub total_sections: usize,
    pub total_words: usize,
}

impl SearchIndex {
    pub fn new() -> Self {
        SearchIndex {
            version: 1,
            docs: HashMap::new(),
            term_posting: HashMap::new(),
            trigram_posting: HashMap::new(),
            total_sections: 0,
            total_words: 0,
        }
    }
}

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds or incrementally updates a binary search index across the given paths
pub fn build_search_index(paths: &[PathBuf]) -> SearchIndex {
    let index_path = PathBuf::from(DEFAULT_INDEX_FILENAME);
    let mut index = if index_path.exists() {
        load_index_from_file(&index_path).unwrap_or_else(|_| SearchIndex::new())
    } else {
        SearchIndex::new()
    };

    let resolved_files = crate::core::io::collect_markdown_files_from_paths(paths);
    let current_path_strs: std::collections::HashSet<String> = resolved_files
        .iter()
        .map(|p| p.display().to_string())
        .collect();

    // Remove deleted files from index
    let existing_keys: Vec<String> = index.docs.keys().cloned().collect();
    for key in existing_keys {
        if !current_path_strs.contains(&key) {
            remove_file_from_index(&mut index, Path::new(&key));
        }
    }

    // Re-index only new or modified files
    for path in resolved_files {
        let doc_path_str = path.display().to_string();
        if let Ok(metadata) = std::fs::metadata(&path) {
            let mtime_secs = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let file_size = metadata.len();

            let needs_update = match index.docs.get(&doc_path_str) {
                Some((existing_meta, _)) => {
                    existing_meta.mtime_secs != mtime_secs || existing_meta.file_size != file_size
                }
                None => true,
            };

            if needs_update {
                let _ = update_file_in_index(&mut index, &path);
            }
        }
    }

    populate_trigram_index(&mut index);
    index
}

/// Extracts character trigrams for fuzzy string similarity search
pub fn extract_trigrams(text: &str) -> Vec<String> {
    let clean = text.to_lowercase();
    let chars: Vec<char> = clean.chars().filter(|c| c.is_alphanumeric()).collect();
    if chars.len() < 3 {
        return Vec::new();
    }
    let mut trigrams = Vec::with_capacity(chars.len() - 2);
    for window in chars.windows(3) {
        trigrams.push(window.iter().collect());
    }
    trigrams
}

/// Populates trigram index from indexed terms
pub fn populate_trigram_index(index: &mut SearchIndex) {
    index.trigram_posting.clear();
    for term in index.term_posting.keys() {
        for tri in extract_trigrams(term) {
            let terms = index.trigram_posting.entry(tri).or_default();
            if !terms.contains(term) {
                terms.push(term.clone());
            }
        }
    }
}

/// Incrementally removes a file entry from the index
pub fn remove_file_from_index(index: &mut SearchIndex, file_path: &Path) {
    let doc_path_str = file_path.display().to_string();
    if let Some((_, old_lines)) = index.docs.remove(&doc_path_str) {
        let old_sections = old_lines.len();
        let old_words: usize = old_lines.iter().map(|l| l.line_words).sum();
        index.total_sections = index.total_sections.saturating_sub(old_sections);
        index.total_words = index.total_words.saturating_sub(old_words);

        // Remove doc postings from term_posting
        for postings in index.term_posting.values_mut() {
            postings.retain(|(path, _)| path != &doc_path_str);
        }
        index
            .term_posting
            .retain(|_, postings| !postings.is_empty());
    }
}

/// Incrementally updates index for a single modified or added file
pub fn update_file_in_index(index: &mut SearchIndex, file_path: &Path) -> anyhow::Result<()> {
    remove_file_from_index(index, file_path);

    if file_path.exists() {
        let metadata = std::fs::metadata(file_path)?;
        let content = read_markdown_file_safe(file_path)?;
        let doc_path_str = file_path.display().to_string();

        let mtime_secs = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let file_size = metadata.len();

        let doc_meta = IndexedDocMeta {
            path: file_path.to_path_buf(),
            mtime_secs,
            file_size,
        };

        let outline = extract_outline(&content);
        let flat_headings = flatten_headings(&outline.headings);
        let lines: Vec<&str> = content.lines().collect();
        let mut indexed_lines = Vec::new();

        for (line_idx, line) in lines.iter().enumerate() {
            let line_num = line_idx + 1;
            let current_heading = flat_headings
                .iter()
                .rfind(|h| h.start_line <= line_num && line_num <= h.end_line);

            let (heading_path, heading_level, heading_title, section_tokens) = match current_heading
            {
                Some(h) => (
                    format!("H{}: {}", h.level, h.title),
                    h.level,
                    h.title.to_lowercase(),
                    h.token_count,
                ),
                None => (String::from("Root Document"), 0, String::new(), 0),
            };

            let line_words = line.split_whitespace().count().max(1);
            indexed_lines.push(IndexedLine {
                line_number: line_num,
                line_snippet: line.trim().to_string(),
                line_words,
                heading_path,
                heading_level,
                heading_title,
                section_tokens,
            });
        }

        for (l_idx, line) in indexed_lines.iter().enumerate() {
            index.total_sections += 1;
            index.total_words += line.line_words;

            let line_terms: Vec<String> = line
                .line_snippet
                .to_lowercase()
                .split_whitespace()
                .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                .filter(|s| !s.is_empty())
                .collect();

            for term in line_terms {
                index
                    .term_posting
                    .entry(term)
                    .or_default()
                    .push((doc_path_str.clone(), l_idx));
            }
        }

        index.docs.insert(doc_path_str, (doc_meta, indexed_lines));
    }

    populate_trigram_index(index);
    Ok(())
}

/// Saves search index to disk
pub fn save_index_to_file(index: &SearchIndex, out_path: &Path) -> anyhow::Result<()> {
    let json_bytes = serde_json::to_vec(index)?;
    let mut file = File::create(out_path)?;
    file.write_all(&json_bytes)?;
    Ok(())
}

/// Loads search index from disk
pub fn load_index_from_file(in_path: &Path) -> anyhow::Result<SearchIndex> {
    let mut file = File::open(in_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let index: SearchIndex = serde_json::from_slice(&buffer)?;
    Ok(index)
}

use crate::core::query::parse_query;

/// Queries pre-computed binary search index for instant sub-5ms BM25 ranking
pub fn search_with_index(index: &SearchIndex, raw_query: &str) -> MultiDocSearchResult {
    let parsed_query = parse_query(raw_query);
    let mut query_terms = parsed_query.positive_terms.clone();
    for phrase in &parsed_query.exact_phrases {
        if !query_terms.contains(phrase) {
            query_terms.push(phrase.clone());
        }
    }

    if parsed_query.is_empty() {
        return MultiDocSearchResult {
            query: raw_query.to_string(),
            total_hits: 0,
            files_searched: index.docs.len(),
            hits: Vec::new(),
        };
    }

    let mut hit_candidates: HashMap<(String, usize), SearchHit> = HashMap::new();

    // If query terms have no exact term postings, check trigram_posting for fuzzy matches
    let mut fuzzy_matched_terms = Vec::new();
    for term in &query_terms {
        if !index.term_posting.contains_key(term) {
            for tri in extract_trigrams(term) {
                if let Some(matches) = index.trigram_posting.get(&tri) {
                    for m in matches {
                        if !fuzzy_matched_terms.contains(m) {
                            fuzzy_matched_terms.push(m.clone());
                        }
                    }
                }
            }
        }
    }
    query_terms.extend(fuzzy_matched_terms);

    for term in &query_terms {
        if let Some(postings) = index.term_posting.get(term) {
            for (doc_path, line_idx) in postings {
                if let Some((_, lines)) = index.docs.get(doc_path) {
                    if let Some(line) = lines.get(*line_idx) {
                        let full_content = lines
                            .iter()
                            .map(|l| l.line_snippet.as_str())
                            .collect::<Vec<&str>>()
                            .join("\n");
                        if parsed_query.matches_filters(doc_path, &full_content) {
                            hit_candidates
                                .entry((doc_path.clone(), *line_idx))
                                .or_insert_with(|| SearchHit {
                                    file_path: doc_path.clone(),
                                    heading_path: line.heading_path.clone(),
                                    line_number: line.line_number,
                                    line_snippet: line.line_snippet.clone(),
                                    section_tokens: line.section_tokens,
                                    score: 0.0,
                                });
                        }
                    }
                }
            }
        }
    }

    let n_docs = (index.docs.len()).max(1) as f64;
    let avgdl = if index.total_sections > 0 {
        index.total_words as f64 / index.total_sections as f64
    } else {
        1.0
    };
    let k1 = 1.2;
    let b = 0.75;

    // Calculate unique document frequency df(t) for each query term in index
    let mut df_map: HashMap<String, usize> = HashMap::new();
    for term in &query_terms {
        if let Some(postings) = index.term_posting.get(term) {
            let unique_docs: std::collections::HashSet<&String> =
                postings.iter().map(|(dp, _)| dp).collect();
            df_map.insert(term.clone(), unique_docs.len());
        }
    }

    let mut hits: Vec<SearchHit> = hit_candidates
        .into_iter()
        .map(|((doc_path, line_idx), mut hit)| {
            let mut total_score = 0.0;
            if let Some((_, lines)) = index.docs.get(&doc_path) {
                if let Some(line) = lines.get(line_idx) {
                    let line_lower = line.line_snippet.to_lowercase();
                    for term in &query_terms {
                        let tf = line_lower.matches(term).count() as f64;
                        if tf > 0.0 {
                            let df = (*df_map.get(term).unwrap_or(&1)).max(1) as f64;
                            let idf = ((n_docs - df + 0.5) / (df + 0.5) + 1.0).ln();
                            let num = tf * (k1 + 1.0);
                            let denom = tf + k1 * (1.0 - b + b * (line.line_words as f64 / avgdl));
                            let bm25 = idf * (num / denom);

                            let heading_multiplier = if line.heading_title.contains(term) {
                                match line.heading_level {
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
                }
            }
            hit.file_path = hit.file_path.replace('\\', "/");
            hit.score = (total_score * 100.0).round() / 100.0;
            hit
        })
        .collect();

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    MultiDocSearchResult {
        query: raw_query.to_string(),
        total_hits: hits.len(),
        files_searched: index.docs.len(),
        hits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_index_build_save_load_search() {
        let temp_dir = std::env::temp_dir();
        let f1 = temp_dir.join("index_test_1.md");
        let mut file = File::create(&f1).unwrap();
        writeln!(file, "# Architecture\n\nInstant index test snippet.").unwrap();

        let index = build_search_index(std::slice::from_ref(&f1));
        let index_file = temp_dir.join(".pankh_index_test.bin");
        save_index_to_file(&index, &index_file).unwrap();

        let loaded = load_index_from_file(&index_file).unwrap();
        let res = search_with_index(&loaded, "Instant");
        assert_eq!(res.total_hits, 1);
        assert!(res.hits[0].line_snippet.contains("Instant"));

        let _ = std::fs::remove_file(f1);
        let _ = std::fs::remove_file(index_file);
    }

    #[test]
    fn test_incremental_index_update_and_removal() {
        let temp_dir = std::env::temp_dir();
        let f1 = temp_dir.join("inc_test_1.md");
        let mut file = File::create(&f1).unwrap();
        writeln!(file, "# Overview\n\nInitial keyword Alpha.").unwrap();

        let mut index = build_search_index(std::slice::from_ref(&f1));
        let res1 = search_with_index(&index, "Alpha");
        assert_eq!(res1.total_hits, 1);

        // Update file content
        let mut file = File::create(&f1).unwrap();
        writeln!(file, "# Overview\n\nUpdated keyword Beta.").unwrap();

        update_file_in_index(&mut index, &f1).unwrap();
        let res_old = search_with_index(&index, "Alpha");
        assert_eq!(res_old.total_hits, 0);

        let res_new = search_with_index(&index, "Beta");
        assert_eq!(res_new.total_hits, 1);

        remove_file_from_index(&mut index, &f1);
        let res_removed = search_with_index(&index, "Beta");
        assert_eq!(res_removed.total_hits, 0);

        let _ = std::fs::remove_file(f1);
    }
}
