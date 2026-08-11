# Dynamic BM25 Document Frequency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement dynamic corpus document frequency $df(t)$ calculation in `src/core/search.rs` to compute accurate Okapi BM25 IDF scores.

**Architecture:** Update `search_documents` in `src/core/search.rs` to build a `df_map: HashMap<String, usize>` across `raw_hits` before applying the BM25 formula.

**Tech Stack:** Rust 2021, `std::collections::HashMap`.

## Global Constraints
- Pre-compute $df(t)$ for each query term $t$.
- All 65+ existing tests must continue passing cleanly with zero `clippy` warnings.

---

### Task 1: Implement Dynamic DF Calculation in `src/core/search.rs`

**Files:**
- Modify: `src/core/search.rs`

**Interfaces:**
- Consumes: `raw_hits`, `query_terms`
- Produces: `df_map: HashMap<String, usize>`, accurate `hit.score`

- [ ] **Step 1: Write failing unit test for dynamic DF scoring**

Add test to `src/core/search.rs`:

```rust
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
```

- [ ] **Step 2: Implement dynamic $df(t)$ calculation in `src/core/search.rs`**

Update `search_documents`:

```rust
    let n_docs = raw_hits.len() as f64;
    let avgdl = if total_sections > 0 { total_words as f64 / total_sections as f64 } else { 1.0 };
    let k1 = 1.2;
    let b = 0.75;

    // Calculate exact document frequency df(t) for each query term
    let mut df_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for term in &query_terms {
        let count = raw_hits
            .iter()
            .filter(|(_, line_lower, _, heading_title, _)| {
                line_lower.contains(term) || heading_title.contains(term)
            })
            .count();
        df_map.insert(term.clone(), count);
    }

    let mut hits: Vec<SearchHit> = raw_hits
        .into_iter()
        .map(|(mut hit, line_lower, heading_level, heading_title, doc_len)| {
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

            hit.score = (total_score * 100.0).round() / 100.0;
            hit
        })
        .collect();
```

- [ ] **Step 3: Run full test suite and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS (0 errors, 0 clippy warnings)

- [ ] **Step 4: Commit**

```bash
git add src/core/search.rs
git commit -m "feat: implement dynamic document frequency df(t) in BM25 relevance scoring"
```
