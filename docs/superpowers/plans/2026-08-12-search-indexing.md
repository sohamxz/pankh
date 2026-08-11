# Pre-computed Search Indexing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `src/core/index.rs` to build, save, load, and query `.pankh_index.bin` for instant sub-5ms BM25 search across 10,000+ file monorepos. Wire `--build-index` CLI flag and auto-index loading in `src/main.rs`.

**Architecture:** Create `src/core/index.rs` exposing `SearchIndex`, `build_search_index`, `save_index_to_file`, `load_index_from_file`, and `search_with_index`. Export `pub mod index;` in `src/core/mod.rs`. Add `--build-index` flag to `Cli` in `src/main.rs`.

**Tech Stack:** Rust 2021, `serde`, `serde_json`.

## Global Constraints
- All 75+ existing tests must continue passing cleanly with zero `clippy` warnings.

---

### Task 1: Create `src/core/index.rs` Indexing Engine

**Files:**
- Create: `src/core/index.rs`
- Modify: `src/core/mod.rs`

- [ ] **Step 1: Write failing unit test for `src/core/index.rs`**

Add test to `src/core/index.rs`:

```rust
#[test]
fn test_index_build_save_load_search() {
    let temp_dir = std::env::temp_dir();
    let f1 = temp_dir.join("index_test_1.md");
    let mut file = std::fs::File::create(&f1).unwrap();
    writeln!(file, "# Architecture\n\nInstant index test snippet.").unwrap();

    let index = build_search_index(&[f1.clone()]);
    let index_file = temp_dir.join(".pankh_index_test.bin");
    save_index_to_file(&index, &index_file).unwrap();

    let loaded = load_index_from_file(&index_file).unwrap();
    let res = search_with_index(&loaded, "Instant");
    assert_eq!(res.total_hits, 1);
    assert!(res.hits[0].line_snippet.contains("Instant"));

    let _ = std::fs::remove_file(f1);
    let _ = std::fs::remove_file(index_file);
}
```

- [ ] **Step 2: Implement `src/core/index.rs`**

- [ ] **Step 3: Export `pub mod index;` in `src/core/mod.rs`**

- [ ] **Step 4: Commit**

```bash
git add src/core/index.rs src/core/mod.rs
git commit -m "feat: implement pre-computed search index engine (build, save, load, query)"
```

---

### Task 2: Wire `--build-index` and Auto-Index Search in `src/main.rs`

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/cli_test.rs`

- [ ] **Step 1: Add `--build-index` flag to `Cli` in `src/main.rs`**

- [ ] **Step 2: Wire auto-index detection in `src/main.rs` search mode**

- [ ] **Step 3: Run full test suite and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS (0 errors, 0 clippy warnings)

- [ ] **Step 4: Commit**

```bash
git add src/main.rs tests/cli_test.rs
git commit -m "feat: wire --build-index flag and auto index loading for search"
```
