# Design Specification: Hyper-Parallel Multi-Document Search Engine (`rayon`)

## Summary
Parallelize Pankh's Okapi BM25 multi-document search engine (`src/core/search.rs`) using `rayon`. Replaces sequential file processing with a multi-threaded parallel iterator (`par_iter`), enabling ultra-fast search execution across thousands of Markdown documents in large monorepos.

---

## 1. Parallel Architecture

### Dependencies (`Cargo.toml`)
Add `rayon = "1.10"`.

### Parallel Iterator Pipeline (`src/core/search.rs`)
```rust
use rayon::prelude::*;

let file_results: Vec<(Vec<RawHit>, usize, usize)> = paths
    .par_iter()
    .filter_map(|path| {
        if let Ok(content) = read_markdown_file_safe(path) {
            // Process file reading, heading outline extraction, and term matching in parallel
            // ...
        } else {
            None
        }
    })
    .collect();
```

---

## 2. Verification Plan

### Automated Unit & Benchmark Tests
- `test_parallel_multi_document_search`: Verifies that parallel BM25 search returns identical top hits as sequential processing across multiple temporary documents.
