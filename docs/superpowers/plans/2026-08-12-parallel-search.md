# Hyper-Parallel Multi-Document Search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate `rayon = "1.10"` in `Cargo.toml` and parallelize `search_documents` in `src/core/search.rs` to process thousands of Markdown documents concurrently.

**Architecture:** Add `rayon = "1.10"` to `Cargo.toml`. Convert document scanning in `search_documents` to `paths.par_iter().filter_map(...)`.

**Tech Stack:** Rust 2021, `rayon`.

## Global Constraints
- Use `rayon::prelude::*`.
- All 73+ existing tests must continue passing cleanly with zero `clippy` warnings.

---

### Task 1: Add `rayon` and Parallelize `search_documents` in `src/core/search.rs`

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/core/search.rs`

- [ ] **Step 1: Add `rayon = "1.10"` to `Cargo.toml`**

- [ ] **Step 2: Write failing unit test for parallel search**

Add test to `src/core/search.rs`:

```rust
#[test]
fn test_parallel_multi_document_search() {
    let temp_dir = std::env::temp_dir();
    let mut paths = Vec::new();
    for i in 0..10 {
        let p = temp_dir.join(format!("parallel_test_{}.md", i));
        let mut f = File::create(&p).unwrap();
        writeln!(f, "# Section {}\n\nContent matching query term in file {}.", i, i).unwrap();
        paths.push(p);
    }

    let results = search_documents(&paths, "query");
    assert_eq!(results.total_hits, 10);
    assert_eq!(results.files_searched, 10);

    for p in paths {
        let _ = std::fs::remove_file(p);
    }
}
```

- [ ] **Step 3: Update `src/core/search.rs` with `rayon` parallel iterator**

```rust
use rayon::prelude::*;

// In search_documents:
let file_results: Vec<(Vec<SearchHitTuple>, usize, usize)> = paths
    .par_iter()
    .filter_map(|path| {
        if let Ok(content) = read_markdown_file_safe(path) {
            // Read, extract outline, scan lines concurrently
            // ...
        } else {
            None
        }
    })
    .collect();
```

- [ ] **Step 4: Run full test suite and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS (0 errors, 0 clippy warnings)

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/core/search.rs
git commit -m "feat: implement hyper-parallel multi-document search engine using rayon"
```
