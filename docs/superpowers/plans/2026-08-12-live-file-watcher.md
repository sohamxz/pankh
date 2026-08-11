# Live File Watcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `src/core/watcher.rs` using `notify` crate to live-reload Markdown files in TUI mode and auto-regenerate `llms.txt` / `llms-full.txt` on document edit when `--watch` flag is provided.

**Architecture:** Add `notify = "6.1"` to `Cargo.toml`. Implement `src/core/watcher.rs` exposing `watch_paths(paths, on_change)`. Wire `--watch` flag into `src/main.rs`, `src/tui/app.rs`, and `src/core/llmstxt.rs`.

**Tech Stack:** Rust 2021, `notify`, `std::sync::mpsc`.

## Global Constraints
- `notify = "6.1"` in `Cargo.toml`.
- `--watch` clap flag in `Cli` struct.
- All 71+ existing tests must continue passing cleanly with zero `clippy` warnings.

---

### Task 1: Create `src/core/watcher.rs` Module with `notify`

**Files:**
- Modify: `Cargo.toml`
- Create: `src/core/watcher.rs`
- Modify: `src/core/mod.rs`

- [ ] **Step 1: Add `notify = "6.1"` to `Cargo.toml`**

- [ ] **Step 2: Implement `src/core/watcher.rs`**

```rust
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn watch_paths<F>(paths: &[PathBuf], mut on_change: F) -> anyhow::Result<()>
where
    F: FnMut() + Send + 'static,
{
    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    for path in paths {
        if path.exists() {
            let mode = if path.is_dir() {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            let _ = watcher.watch(path, mode);
        }
    }

    std::thread::spawn(move || {
        let _watcher = watcher;
        while let Ok(res) = rx.recv() {
            match res {
                Ok(Event { kind, .. }) => match kind {
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                        std::thread::sleep(Duration::from_millis(100));
                        on_change();
                    }
                    _ => {}
                },
                Err(_) => break,
            }
        }
    });

    Ok(())
}
```

- [ ] **Step 3: Export `pub mod watcher;` in `src/core/mod.rs`**

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml src/core/watcher.rs src/core/mod.rs
git commit -m "feat: implement file watcher module using notify crate"
```

---

### Task 2: Wire `--watch` Flag in TUI Reader and `llms.txt` Engine

**Files:**
- Modify: `src/main.rs`
- Modify: `src/tui/app.rs`
- Modify: `tests/cli_test.rs`

- [ ] **Step 1: Add `--watch` flag to `Cli` in `src/main.rs`**

```rust
    /// Watch file(s) or directory for changes and live-reload TUI or auto-regenerate llms.txt
    #[arg(short, long)]
    pub watch: bool,
```

- [ ] **Step 2: Wire watch loop in `--llms-txt` and TUI reader in `src/main.rs` & `src/tui/app.rs`**

- [ ] **Step 3: Run full test suite and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS (0 errors, 0 clippy warnings)

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/tui/app.rs tests/cli_test.rs
git commit -m "feat: wire --watch flag for TUI live-reload and llms.txt auto-regeneration"
```
