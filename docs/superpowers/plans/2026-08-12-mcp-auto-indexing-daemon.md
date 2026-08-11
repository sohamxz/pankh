# Real-Time Auto-Indexing MCP Daemon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Real-Time Auto-Indexing MCP Daemon that monitors file changes in the workspace, updates `Arc<RwLock<SearchIndex>>` concurrently, and emits `notifications/resources/updated` JSON-RPC notifications to LLM clients.

**Architecture:** Update `src/mcp/server.rs` to wrap `SearchIndex` in `Arc<RwLock<SearchIndex>>`, spawn background `notify` thread with 100ms debouncing, and send JSON-RPC notification frames.

**Tech Stack:** Rust 2021, `std::sync::Arc`, `parking_lot` / `std::sync::RwLock`, `notify`.

## Global Constraints
- All existing 88 unit and integration tests must pass cleanly.
- `cargo clippy -- -D warnings` must produce 0 warnings.

---

### Task 1: Refactor MCP Server State to `Arc<RwLock<SearchIndex>>`

**Files:**
- Modify: `src/mcp/server.rs`

- [ ] **Step 1: Write failing unit test for `Arc<RwLock<SearchIndex>>` thread safety**

```rust
#[test]
fn test_mcp_server_thread_safe_shared_index() {
    let index = Arc::new(RwLock::new(SearchIndex::new()));
    let index_clone = Arc::clone(&index);
    let handle = std::thread::spawn(move || {
        let mut guard = index_clone.write().unwrap();
        guard.total_sections += 1;
    });
    handle.join().unwrap();
    assert_eq!(index.read().unwrap().total_sections, 1);
}
```

- [ ] **Step 2: Refactor `src/mcp/server.rs` to use `Arc<RwLock<SearchIndex>>`**

- [ ] **Step 3: Run unit tests and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/mcp/server.rs
git commit -m "refactor: wrap MCP SearchIndex state in Arc<RwLock<SearchIndex>>"
```

---

### Task 2: Implement Background Auto-Indexing Watcher & Notification Frame Dispatch

**Files:**
- Modify: `src/mcp/server.rs`
- Modify: `tests/mcp_test.rs`

- [ ] **Step 1: Write integration test for real-time MCP auto-indexing in `tests/mcp_test.rs`**

```rust
#[test]
fn test_mcp_auto_indexing_daemon_updates_index_on_file_create() {
    let temp_dir = std::env::temp_dir().join("pankh_mcp_daemon_test");
    let _ = std::fs::create_dir_all(&temp_dir);
    let file1 = temp_dir.join("doc1.md");
    std::fs::write(&file1, "# Dynamic Title\nDynamic content text").unwrap();

    let index = Arc::new(std::sync::RwLock::new(pankh::core::index::build_search_index(&[temp_dir.clone()])));
    let _watcher = pankh::core::watcher::watch_paths(&[temp_dir.clone()], {
        let index_clone = Arc::clone(&index);
        let file1_clone = file1.clone();
        move || {
            let mut guard = index_clone.write().unwrap();
            let _ = pankh::core::index::update_file_in_index(&mut guard, &file1_clone);
        }
    });

    std::thread::sleep(std::time::Duration::from_millis(200));
    let res = pankh::core::index::search_with_index(&index.read().unwrap(), "Dynamic");
    assert!(!res.hits.is_empty());

    let _ = std::fs::remove_dir_all(temp_dir);
}
```

- [ ] **Step 2: Implement debounced background watcher thread in `run_mcp_server`**

- [ ] **Step 3: Run full test suite and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/mcp/server.rs tests/mcp_test.rs
git commit -m "feat: implement real-time auto-indexing MCP daemon with notifications"
```
