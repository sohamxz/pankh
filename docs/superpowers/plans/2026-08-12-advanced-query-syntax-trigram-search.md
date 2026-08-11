# Advanced Query Syntax & Trigram Fuzzy Search Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Boolean Query Syntax Parser (`path:`, `ext:`, `lang:`, `-negation`, `"exact phrase"`) and Trigram Fuzzy Search Engine integrated across Pankh's CLI, TUI, Indexer, and MCP server.

**Architecture:** Create `src/core/query.rs` query parser, extend `SearchIndex` in `src/core/index.rs` with `trigram_posting`, and update `search_documents` & `search_with_index` to evaluate query AST filters and trigram fuzzy matching.

**Tech Stack:** Rust 2021, `serde`, `bincode`, `rayon`.

## Global Constraints
- All existing 84 unit and integration tests must pass cleanly.
- `cargo clippy -- -D warnings` must produce 0 warnings.

---

### Task 1: Create Query AST Parser (`src/core/query.rs`)

**Files:**
- [NEW] `src/core/query.rs`
- Modify: `src/core/mod.rs`

- [ ] **Step 1: Write failing unit test for Query Parser**

```rust
#[test]
fn test_parse_query_syntax() {
    let q = parse_query("path:docs/ lang:rs -deprecated \"exact phrase\" term");
    assert_eq!(q.field_filters.len(), 2);
    assert_eq!(q.negated_terms, vec!["deprecated"]);
    assert_eq!(q.exact_phrases, vec!["exact phrase"]);
    assert_eq!(q.positive_terms, vec!["term"]);
}
```

- [ ] **Step 2: Implement `src/core/query.rs`**

- [ ] **Step 3: Run unit tests and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/core/query.rs src/core/mod.rs
git commit -m "feat: implement boolean query syntax parser"
```

---

### Task 2: Extend `SearchIndex` with Trigram Postings (`src/core/index.rs`)

**Files:**
- Modify: `src/core/index.rs`

- [ ] **Step 1: Write failing unit test for Trigram Indexing**

```rust
#[test]
fn test_trigram_fuzzy_index_lookup() {
    let index = build_search_index(&[PathBuf::from("tests/fixtures/sample.md")]);
    let matches = search_trigrams(&index, "algoritm");
    assert!(!matches.is_empty());
}
```

- [ ] **Step 2: Add `trigram_posting` to `SearchIndex` and update binary serializer**

- [ ] **Step 3: Run tests and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/core/index.rs
git commit -m "feat: add trigram fuzzy indexing to SearchIndex"
```

---

### Task 3: Wire Advanced Query Engine into CLI, TUI, and MCP

**Files:**
- Modify: `src/core/search.rs`
- Modify: `src/main.rs`
- Modify: `src/mcp/server.rs`

- [ ] **Step 1: Integrate `parse_query` into `search_documents` and `search_with_index`**

- [ ] **Step 2: Run full test suite and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/core/search.rs src/main.rs src/mcp/server.rs
git commit -m "feat: wire advanced query syntax and trigram fuzzy search into CLI, TUI, and MCP"
```
