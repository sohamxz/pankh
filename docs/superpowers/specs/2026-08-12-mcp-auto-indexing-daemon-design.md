# Design Specification: Real-time Auto-Indexing MCP Daemon

## Executive Summary
Pankh (🪶)'s Model Context Protocol (MCP) server is upgrading from static index reading to a **Real-Time Auto-Indexing MCP Daemon**. When running `pankh mcp`, the server spawns a debounced background watcher thread (`notify`) that monitors workspace Markdown file modifications, additions, and deletions, updating an `Arc<RwLock<SearchIndex>>` in sub-1ms and emitting `notifications/resources/updated` JSON-RPC notifications to connected LLM clients (e.g. Cursor, Claude Desktop).

---

## 1. Concurrent State & Threading Model (`src/mcp/server.rs`)

### 1.1 Shared Index State
```rust
pub struct ServerState {
    pub index: Arc<RwLock<SearchIndex>>,
    pub workspace_root: PathBuf,
}
```

### 1.2 Debounced Watcher Thread
- On `run_mcp_server` initialization, spawn a `notify` watcher thread targeting `workspace_root`.
- When file events fire (`Modify`, `Create`, `Remove`), debounce by 100ms.
- Acquire write lock on `Arc<RwLock<SearchIndex>>` and invoke `update_file_in_index` or `remove_file_from_index`.
- Save updated binary index to `.pankh_index.bin`.
- Emit JSON-RPC frame over stdout:
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/resources/updated",
  "params": {
    "uri": "file:///path/to/modified.md"
  }
}
```

---

## 2. Tool & Resource Handler Upgrades

- `search_sections` tool acquires read lock `index.read()` for non-blocking sub-1ms BM25 + Query Filter evaluation while background writes take place seamlessly.
- `read_clean` and `chunk_markdown` read fresh file content directly.

---

## 3. Verification Plan

- **Integration Test (`tests/mcp_test.rs`):** Start MCP server state, write a new markdown file to temporary directory, wait 200ms for debounce, and assert `search_sections` tool call returns hits for the new file.
- **Unit Test (`src/mcp/server.rs`):** Verify `Arc<RwLock<SearchIndex>>` concurrent reader/writer access.
