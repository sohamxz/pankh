# Incremental Indexing & Release Engineering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `update_file_in_index` and `remove_file_from_index` in `src/core/index.rs` for sub-1ms incremental search index updates, wire `--build-index --watch`, and create `.github/workflows/release.yml` for multi-platform GitHub releases.

**Architecture:** Extend `src/core/index.rs` with incremental file mutation methods. Wire `--watch` for index rebuilding in `src/main.rs`. Create `.github/workflows/release.yml`.

**Tech Stack:** Rust 2021, GitHub Actions.

## Global Constraints
- All 77+ existing tests must continue passing cleanly with zero `clippy` warnings.

---

### Task 1: Implement Incremental Index Mutation in `src/core/index.rs`

**Files:**
- Modify: `src/core/index.rs`

- [ ] **Step 1: Write failing unit test for incremental index update & removal**

Add test to `src/core/index.rs`:

```rust
#[test]
fn test_incremental_index_update_and_removal() {
    let temp_dir = std::env::temp_dir();
    let f1 = temp_dir.join("inc_test_1.md");
    let mut file = File::create(&f1).unwrap();
    writeln!(file, "# Overview\n\nInitial keyword Alpha.").unwrap();

    let mut index = build_search_index(&[f1.clone()]);
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
```

- [ ] **Step 2: Implement `update_file_in_index` and `remove_file_from_index` in `src/core/index.rs`**

- [ ] **Step 3: Run full test suite and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS (0 errors, 0 clippy warnings)

- [ ] **Step 4: Commit**

```bash
git add src/core/index.rs
git commit -m "feat: implement incremental index file update and removal"
```

---

### Task 2: Create GitHub Actions Release Workflow `.github/workflows/release.yml`

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create `.github/workflows/release.yml` for multi-platform binary compilation**

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add GitHub Actions release workflow for Windows, macOS, and Linux binary releases"
```
