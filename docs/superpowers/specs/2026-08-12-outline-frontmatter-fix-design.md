# Design Specification: Fix Outline Tree Section Text Frontmatter Offset Shift

## Summary
Fix heading section text and token count slicing in `src/core/agent.rs` (`build_nested_heading_tree` & `extract_outline`) by passing full raw file lines to `build_nested_heading_tree` instead of frontmatter-stripped body lines.

---

## 1. Bug Analysis & Fix

### Problem
`extract_outline` called `build_nested_heading_tree` passing `lines` created from `content_without_frontmatter.lines()`. However, `FlatHeading.start_line` included `frontmatter_offset`. Slicing body lines at `start_line` caused a shift of `frontmatter_offset` lines down the body text.

### Fix (`src/core/agent.rs`)
Pass `raw_lines` (the full raw lines of `input`) to `build_nested_heading_tree`.

```rust
let raw_lines: Vec<&str> = input.lines().collect();
let headings = build_nested_heading_tree(&flat_headings, &raw_lines, raw_lines.len());
```

---

## 2. Verification Plan

### Automated Unit Test
- `test_frontmatter_outline_section_tokens`: Verifies that a document with frontmatter headers produces accurate section token count metrics for all headings in `extract_outline`.
