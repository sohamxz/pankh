# Frontmatter Line Offset Compensation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `get_frontmatter_line_count` in `src/core/agent.rs` and offset heading `start_line` / `end_line` in `extract_outline` so TUI anchor jumps land on the exact line even when frontmatter exists.

**Architecture:** Add `get_frontmatter_line_count(input)` in `src/core/agent.rs`. Update `extract_outline` to add frontmatter offset to heading start/end lines.

**Tech Stack:** Rust 2021.

## Global Constraints
- `get_frontmatter_line_count(input: &str) -> usize`.
- Add offset to heading line numbers in `extract_outline`.
- All 67+ existing tests must continue passing cleanly with zero `clippy` warnings.

---

### Task 1: Implement Frontmatter Line Offset in `src/core/agent.rs`

**Files:**
- Modify: `src/core/agent.rs`

- [ ] **Step 1: Write failing unit test for frontmatter line offset compensation**

Add test to `src/core/agent.rs`:

```rust
#[test]
fn test_frontmatter_line_offset_compensation() {
    let raw = "---\ntitle: Document\nauthor: Test\n---\n\n# Main Title\n\nContent here.";
    let outline = extract_outline(raw);
    assert_eq!(outline.headings.len(), 1);
    assert_eq!(outline.headings[0].start_line, 6); // Line 6 in raw file
}
```

- [ ] **Step 2: Implement `get_frontmatter_line_count` and update `extract_outline`**

Add helper function to `src/core/agent.rs`:

```rust
pub fn get_frontmatter_line_count(input: &str) -> usize {
    let trimmed = input.trim_start();
    if trimmed.starts_with("---") {
        if let Some(rest) = trimmed.strip_prefix("---") {
            if let Some(end_idx) = rest.find("\n---") {
                let header_slice = &trimmed[..end_idx + 4];
                return header_slice.lines().count();
            }
        }
    } else if trimmed.starts_with("+++") {
        if let Some(rest) = trimmed.strip_prefix("+++") {
            if let Some(end_idx) = rest.find("\n+++") {
                let header_slice = &trimmed[..end_idx + 4];
                return header_slice.lines().count();
            }
        }
    }
    0
}
```

Update `extract_outline`:

```rust
    let frontmatter_offset = get_frontmatter_line_count(input);
    let content_without_frontmatter = strip_frontmatter(input);
    let lines: Vec<&str> = content_without_frontmatter.lines().collect();
    let total_lines = input.lines().count();

    // In flat heading scan:
    start_line: line_idx + 1 + frontmatter_offset,
```

- [ ] **Step 3: Run full test suite and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS (0 errors, 0 clippy warnings)

- [ ] **Step 4: Commit**

```bash
git add src/core/agent.rs
git commit -m "fix: compensate frontmatter line offset in heading outline extraction"
```
