# Outline Tree Section Text Frontmatter Offset Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix line slicing in `extract_outline` (`src/core/agent.rs`) when documents contain YAML/TOML frontmatter headers by passing full raw lines to `build_nested_heading_tree`.

**Architecture:** Pass `raw_lines` (`input.lines().collect()`) to `build_nested_heading_tree` in `src/core/agent.rs`.

**Tech Stack:** Rust 2021.

## Global Constraints
- All existing tests must continue passing cleanly with zero `clippy` warnings.

---

### Task 1: Fix `extract_outline` Line Slicing in `src/core/agent.rs`

**Files:**
- Modify: `src/core/agent.rs`

- [ ] **Step 1: Write failing unit test for frontmatter outline section token metrics**

Add test to `src/core/agent.rs`:

```rust
#[test]
fn test_frontmatter_outline_section_tokens() {
    let raw = "---\ntitle: Doc\nauthor: Tester\n---\n\n# Main Title\n\nSection body text with words.";
    let outline = extract_outline(raw);
    assert_eq!(outline.headings.len(), 1);
    assert!(outline.headings[0].token_count > 0);
    assert_eq!(outline.headings[0].start_line, 6);
}
```

- [ ] **Step 2: Update `extract_outline` in `src/core/agent.rs`**

```rust
let raw_lines: Vec<&str> = input.lines().collect();
let headings = build_nested_heading_tree(&flat_headings, &raw_lines, raw_lines.len());
```

- [ ] **Step 3: Run full test suite and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS (0 errors, 0 clippy warnings)

- [ ] **Step 4: Commit**

```bash
git add src/core/agent.rs
git commit -m "fix: pass full raw file lines to build_nested_heading_tree in extract_outline"
```
