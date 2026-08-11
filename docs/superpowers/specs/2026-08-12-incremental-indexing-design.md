# Design Specification: Incremental Indexing & Release Engineering

## Summary
Implement sub-1ms incremental search index updates (`src/core/index.rs`) to update single-file index entries on disk when modified, and configure GitHub Actions CI release workflow (`.github/workflows/release.yml`) for multi-platform binary compilation.

---

## 1. Incremental Indexing Architecture (`src/core/index.rs`)

### API additions
```rust
/// Incrementally updates index for a single modified or added file
pub fn update_file_in_index(index: &mut SearchIndex, file_path: &Path) -> anyhow::Result<()>

/// Incrementally removes deleted file entry from index
pub fn remove_file_from_index(index: &mut SearchIndex, file_path: &Path)
```

### Algorithm
1. Locate old postings for `file_path` in `index.term_posting` and retain non-matching entries.
2. Deduct old line counts and word counts from `total_sections` and `total_words`.
3. If file exists, read content via `read_markdown_file_safe`, extract outline, parse line terms, and append new postings.
4. Update `index.docs`.

---

## 2. GitHub Actions CI & Release Pipeline (`.github/workflows/release.yml`)

### Targets
- `x86_64-pc-windows-msvc` (`.exe`)
- `x86_64-apple-darwin` & `aarch64-apple-darwin` (macOS Intel & Apple Silicon)
- `x86_64-unknown-linux-gnu` (Linux)

---

## 3. Verification Plan

### Automated Unit Tests
- `test_incremental_index_update_and_removal`: Verifies building an initial index, updating a single modified file, removing a deleted file, saving, and verifying search correctness.
