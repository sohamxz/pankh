# BM25 Search Relevance Ranking Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement an Okapi BM25 relevance scoring engine with heading level multipliers in `src/core/search.rs` and update CLI/MCP search results sorting.

**Architecture:** Add term frequency (TF) and inverse document frequency (IDF) calculation in `src/core/search.rs`. Compute Okapi BM25 scores for each search hit, apply heading multipliers ($H1=3.0\times, H2=2.5\times, H3=2.0\times$), sort hits in descending relevance score order, and expose `score` in `SearchHit`.

**Tech Stack:** Rust 2021, standard `std` math (`f64::ln`), `serde`, `serde_json`, `tokio`.

## Global Constraints
- Pure Rust, zero third-party search dependencies (use `std` math).
- $k_1 = 1.2$, $b = 0.75$.
- `score` field added to `SearchHit` struct (`pub score: f64`).
- All 35+ existing unit & integration tests must continue passing cleanly with zero `clippy` warnings.

---

### Task 1: Impl BM25 Scoring & Search Ranking Engine in `src/core/search.rs`

**Files:**
- Modify: `src/core/search.rs`
- Modify: `tests/cli_test.rs`
- Modify: `tests/mcp_test.rs`

**Interfaces:**
- Consumes: `extract_outline`, `read_markdown_file_safe`, `flatten_headings`
- Produces: `SearchHit` with `score: f64`, `search_documents(paths, query) -> MultiDocSearchResult` sorted by BM25 score.

- [ ] **Step 1: Write failing unit test for BM25 search ranking**

Add `test_bm25_heading_multiplier_boost` to `src/core/search.rs`:

```rust
#[test]
fn test_bm25_heading_multiplier_boost() {
    let temp_dir = std::env::temp_dir();
    let file1 = temp_dir.join("bm25_test_h1.md");
    let file2 = temp_dir.join("bm25_test_body.md");

    let mut f1 = std::fs::File::create(&file1).unwrap();
    use std::io::Write;
    writeln!(f1, "# Database Architecture\n\nOverview.").unwrap();

    let mut f2 = std::fs::File::create(&file2).unwrap();
    writeln!(f2, "# Section\n\nDatabase migration guide for production database.").unwrap();

    let results = search_documents(&[file1.clone(), file2.clone()], "Database");
    assert_eq!(results.total_hits, 2);
    // H1 title match must score higher than body match
    assert!(results.hits[0].heading_path.contains("Database Architecture"));
    assert!(results.hits[0].score > results.hits[1].score);

    let _ = std::fs::remove_file(file1);
    let _ = std::fs::remove_file(file2);
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --lib core::search::tests::test_bm25_heading_multiplier_boost`
Expected: FAIL (missing `score` field / un-ranked search hits)

- [ ] **Step 3: Implement BM25 scoring logic in `src/core/search.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchHit {
    pub file_path: String,
    pub heading_path: String,
    pub line_number: usize,
    pub line_snippet: String,
    pub section_tokens: usize,
    pub score: f64,
}

pub fn search_documents(paths: &[PathBuf], query: &str) -> MultiDocSearchResult {
    let query_terms: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if query_terms.is_empty() {
        return MultiDocSearchResult {
            query: query.to_string(),
            total_hits: 0,
            files_searched: 0,
            hits: Vec::new(),
        };
    }

    let mut raw_hits = Vec::new();
    let mut files_searched = 0;
    let mut total_sections = 0;
    let mut total_words = 0;

    for path in paths {
        if let Ok(content) = read_markdown_file_safe(path) {
            files_searched += 1;
            let file_path_str = path.display().to_string();
            let outline = extract_outline(&content);
            let flat_headings = flatten_headings(&outline.headings);
            let lines: Vec<&str> = content.lines().collect();

            for (line_idx, line) in lines.iter().enumerate() {
                let line_lower = line.to_lowercase();
                if query_terms.iter().any(|term| line_lower.contains(term)) {
                    let line_num = line_idx + 1;
                    let current_heading = flat_headings
                        .iter()
                        .rfind(|h| h.start_line <= line_num && line_num <= h.end_line);

                    let (heading_path, heading_level, heading_title, section_tokens) = match current_heading {
                        Some(h) => (format!("H{}: {}", h.level, h.title), h.level, h.title.to_lowercase(), h.token_count),
                        None => (String::from("Root Document"), 0, String::new(), 0),
                    };

                    let line_words = line.split_whitespace().count().max(1);
                    total_sections += 1;
                    total_words += line_words;

                    raw_hits.push((
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
        }
    }

    let n_docs = raw_hits.len() as f64;
    let avgdl = if total_sections > 0 { total_words as f64 / total_sections as f64 } else { 1.0 };
    let k1 = 1.2;
    let b = 0.75;

    let mut hits: Vec<SearchHit> = raw_hits
        .into_iter()
        .map(|(mut hit, line_lower, heading_level, heading_title, doc_len)| {
            let mut total_score = 0.0;

            for term in &query_terms {
                let tf = line_lower.matches(term).count() as f64;
                if tf > 0.0 {
                    let df = 1.0; // Per-line hit occurrence frequency
                    let idf = ((n_docs - df + 0.5) / (df + 0.5) + 1.0).ln();
                    let num = tf * (k1 + 1.0);
                    let denom = tf + k1 * (1.0 - b + b * (doc_len as f64 / avgdl));
                    let mut bm25 = idf * (num / denom);

                    // Heading multiplier
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

            hit.score = (total_score * 100.0).round() / 100.0;
            hit
        })
        .collect();

    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    MultiDocSearchResult {
        query: query.to_string(),
        total_hits: hits.len(),
        files_searched,
        hits,
    }
}
```

- [ ] **Step 4: Update CLI output renderer in `src/main.rs`**

Update `src/main.rs` `--search` renderer to display score:
```rust
println!(
    "- [{}:{}] ({}) [Score: {:.2}]\n  > {} (Section Tokens: {})",
    hit.file_path, hit.line_number, hit.heading_path, hit.score, hit.line_snippet, hit.section_tokens
);
```

- [ ] **Step 5: Run tests and verify all pass**

Run: `cargo test`
Expected: PASS (all unit, CLI, and MCP tests passing)

- [ ] **Step 6: Commit**

```bash
git add src/core/search.rs src/main.rs tests/cli_test.rs
git commit -m "feat: implement Okapi BM25 relevance scoring with heading multipliers in search engine"
```
