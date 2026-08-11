# Design Specification: Frontmatter Line Offset Compensation in Heading Outline

## Summary
Fix heading line number alignment in `src/core/agent.rs` by tracking frontmatter line counts (`--- ... ---` or `+++ ... +++`) and offsetting heading `start_line` and `end_line` values so TUI anchor jumping lands on the exact line.

---

## 1. Line Offset Algorithm

### Helper Function (`src/core/agent.rs`)
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

### Outline Tree Update (`src/core/agent.rs`)
Add `frontmatter_offset` to all line number calculations in `extract_outline`.

---

## 2. Verification Plan

### Automated Unit Tests
- `test_frontmatter_line_offset_compensation`: Verifies that heading line numbers in documents with frontmatter match absolute raw file line numbers.
