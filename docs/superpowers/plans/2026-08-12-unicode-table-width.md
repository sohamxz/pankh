# Unicode Table Width Alignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate `unicode-width` crate in `Cargo.toml` and update `src/tui/render.rs` to render visually aligned TUI table border grids for multi-byte Unicode content.

**Architecture:** Add `unicode-width = "0.1"` to `Cargo.toml`. Replace byte length `cell.len()` with `UnicodeWidthStr::width` in `src/tui/render.rs`.

**Tech Stack:** Rust 2021, `unicode-width`.

## Global Constraints
- Use `unicode_width::UnicodeWidthStr::width`.
- All 69+ existing tests must continue passing cleanly with zero `clippy` warnings.

---

### Task 1: Add `unicode-width` and Update TUI Table Rendering in `src/tui/render.rs`

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/tui/render.rs`

- [ ] **Step 1: Add `unicode-width = "0.1"` to `Cargo.toml`**

- [ ] **Step 2: Write failing unit test for Unicode table rendering**

Add test to `src/tui/render.rs`:

```rust
#[test]
fn test_rich_render_unicode_table_alignment() {
    let raw = "| Icon | Name |\n|---|---|\n| 🪶 | Pankh Reader |";
    let lines = render_rich_markdown(raw, "");
    assert!(lines.iter().any(|l| l.to_string().contains("┌")));
    assert!(lines.iter().any(|l| l.to_string().contains("🪶")));
}
```

- [ ] **Step 3: Update `src/tui/render.rs` to use `UnicodeWidthStr::width`**

```rust
use unicode_width::UnicodeWidthStr;

// In table calculation:
col_widths[c_idx] = col_widths[c_idx].max(cell.width());

// In row padding:
let cell_width = cell.width();
let padding = " ".repeat(width.saturating_sub(cell_width));
let padded = format!(" {}{} ", cell_text, padding);
```

- [ ] **Step 4: Run full test suite and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS (0 errors, 0 clippy warnings)

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/tui/render.rs
git commit -m "feat: implement unicode display width alignment for TUI table grid rendering"
```
