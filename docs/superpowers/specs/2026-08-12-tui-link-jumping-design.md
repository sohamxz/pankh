# Design Specification: Interactive Link Jumping & File History in TUI

## Summary
Add interactive Markdown link navigation to Pankh's TUI mode (`src/tui/app.rs`). Pressing `Enter` when on a line with a Markdown link jumps to section anchors (`#heading`) or loads relative files (`doc.md`), with `Backspace` history backtracking.

---

## 1. Interaction & Keyboard Controls

### Keybindings
- **`Enter`**: Follows the highlighted link on the current line.
  - `#section`: Jumps `scroll_offset` directly to section start line.
  - `doc.md` / `path/to/file.md`: Loads new Markdown file in TUI.
- **`Backspace`** / **`u`**: Navigates back to the previous file in `history_stack`.
- **`Tab`** (when link focused): Cycles selected link if multiple links exist on the same line.

---

## 2. Component Architecture

### Data Structures (`src/tui/app.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLink {
    pub line_number: usize,
    pub label: String,
    pub url: String,
}

pub struct App {
    // Existing fields...
    pub links: Vec<DocumentLink>,
    pub selected_link_index: usize,
    pub history_stack: Vec<(String, String, u16)>, // (raw_text, file_path_or_title, scroll_offset)
}
```

### Link Parsing (`src/tui/app.rs`)
Extracts links `[label](url)` using AST/regex parsing and maps them to document line numbers.

### Anchor & File Navigation Logic
- **Anchor Matching:** Normalizes `#` anchor titles (e.g. `#installation` $\rightarrow$ `Installation`), matches against `app.headings`, and updates `scroll_offset`.
- **File Navigation:** Uses `read_markdown_file_safe` to load file targets and pushes state to `history_stack`.

---

## 3. Verification Plan

### Automated Unit Tests
- `test_tui_anchor_link_jump`: Verifies `scroll_offset` jumps to target section line.
- `test_tui_file_history_backtrack`: Verifies file loading and `history_stack` backtracking.
