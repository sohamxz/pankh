# Design Specification: Unicode Table Display Width Alignment in TUI

## Summary
Refine Markdown table grid rendering in `src/tui/render.rs` using `unicode-width` (`UnicodeWidthStr::width`) to calculate exact visual terminal column widths for Unicode emojis, CJK characters, and Unicode symbols.

---

## 1. Unicode Display Width Algorithm

### Dependencies (`Cargo.toml`)
Add `unicode-width = "0.1"`.

### Column Width & Padding Calculation (`src/tui/render.rs`)
Replace `cell.len()` with `unicode_width::UnicodeWidthStr::width(cell.as_str())`.

```rust
use unicode_width::UnicodeWidthStr;

// Column width determination:
col_widths[c_idx] = col_widths[c_idx].max(cell.width());

// Padded cell formatting:
let cell_width = cell.width();
let padding = " ".repeat(target_width.saturating_sub(cell_width));
let padded = format!(" {}{} ", cell, padding);
```

---

## 2. Verification Plan

### Automated Unit Tests
- `test_rich_render_unicode_table_alignment`: Verifies that Markdown tables with multi-byte Unicode characters (e.g., `🪶`, `✓`) render aligned TUI border grids.
