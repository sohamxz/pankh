# Design Specification: Live File Watcher (`pankh --watch`)

## Summary
Implement cross-platform Live File Watcher functionality (`src/core/watcher.rs`) using `notify`. Supports `--watch` CLI flag to live-reload Markdown files in TUI mode without losing scroll position, and auto-regenerate `llms.txt` / `llms-full.txt` on document edit.

---

## 1. Component Architecture & Workflow

### Data Structures & API (`src/core/watcher.rs`)
```rust
pub fn watch_file_targets<F>(paths: &[PathBuf], mut on_change: F) -> anyhow::Result<()>
where
    F: FnMut() + Send + 'static,
```

### 2. Modes of Operation

#### Mode 1: TUI Live Reload (`pankh file.md --watch`)
- Spawns background `notify` channel listener in `src/tui/app.rs`.
- Sends custom crossterm event or updates shared state flag `app.needs_reload`.
- Re-reads file content via `read_markdown_file_safe`, re-calculates headings, links, stats, and preserves `scroll_offset`.

#### Mode 2: `llms.txt` Auto-Regeneration (`pankh --llms-txt docs/ --watch`)
- Watches directory tree `docs/`.
- Re-executes `generate_llmstxt(paths)` on filesystem modifications and updates `llms.txt` / `llms-full.txt` on disk.

---

## 3. Verification Plan

### Automated Unit & Integration Tests
- `test_watcher_file_change_detection`: Verifies file change callback triggering on temporary file write.
- `test_cli_watch_flag`: Verifies `--watch` clap flag parsing.
