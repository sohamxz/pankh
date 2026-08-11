# TUI Interactive Link Jumping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Markdown link extraction, anchor `#heading` jumping, relative file loading, and `Backspace` history navigation in Pankh's TUI (`src/tui/app.rs`).

**Architecture:** Add `DocumentLink` struct and `history_stack` vector to `App` in `src/tui/app.rs`. On document initialization/reload, extract all links `[label](url)` with line numbers. When `Enter` is pressed on a line containing a link, jump `scroll_offset` to anchor `#heading` line or load relative file using `read_markdown_file_safe`. Allow `Backspace` to backtrack to previous file in `history_stack`.

**Tech Stack:** Rust 2021, `pulldown-cmark`, `ratatui`, `crossterm`, `arboard`.

## Global Constraints
- `DocumentLink` struct with `line_number`, `label`, `url`.
- `history_stack: Vec<(String, String, u16)>` in `App`.
- `Enter` triggers anchor jump or file load; `Backspace` backtracks history stack.
- All 54+ existing tests must continue passing cleanly with zero `clippy` warnings.

---

### Task 1: Implement Link Extraction, Anchor Jump & File History in `src/tui/app.rs`

**Files:**
- Modify: `src/tui/app.rs`
- Modify: `src/tui/ui.rs`

**Interfaces:**
- Consumes: `extract_outline`, `clean_markdown`, `read_markdown_file_safe`
- Produces: `DocumentLink`, `App::follow_link()`, `App::backtrack_history()`

- [ ] **Step 1: Write failing unit test for anchor jumping and history backtracking**

Add tests to `src/tui/app.rs`:

```rust
#[test]
fn test_tui_anchor_link_jump() {
    let content = "# Overview\n\nSee [Installation](#installation) below.\n\n## Installation\n\nDetails here.";
    let mut app = App::new(content);
    assert_eq!(app.links.len(), 1);
    assert_eq!(app.links[0].url, "#installation");

    app.scroll_offset = 2;
    app.follow_current_line_link();
    assert_eq!(app.scroll_offset, 4); // Line 5 (0-indexed 4) for ## Installation
}

#[test]
fn test_tui_file_history_backtrack() {
    let mut app = App::new("# Doc 1\n\nContent 1");
    app.load_new_document("# Doc 2\n\nContent 2", "doc2.md");
    assert_eq!(app.history_stack.len(), 1);
    assert!(app.raw_text.contains("Doc 2"));

    app.backtrack_history();
    assert_eq!(app.history_stack.len(), 0);
    assert!(app.raw_text.contains("Doc 1"));
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --lib tui::app::tests::test_tui_anchor_link_jump`
Expected: FAIL (missing `follow_current_line_link` & `history_stack`)

- [ ] **Step 3: Implement link extraction and navigation in `src/tui/app.rs`**

Add `DocumentLink` and link parser to `src/tui/app.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLink {
    pub line_number: usize,
    pub label: String,
    pub url: String,
}

pub fn extract_document_links(content: &str) -> Vec<DocumentLink> {
    let parser = pulldown_cmark::Parser::new(content);
    let mut links = Vec::new();
    let mut in_link = false;
    let mut current_label = String::new();
    let mut current_url = String::new();
    let mut line_counter = 1;

    for event in parser {
        match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { dest_url, .. }) => {
                in_link = true;
                current_label.clear();
                current_url = dest_url.to_string();
            }
            pulldown_cmark::Event::Text(text) => {
                line_counter += text.matches('\n').count();
                if in_link {
                    current_label.push_str(&text);
                }
            }
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Link) if in_link => {
                links.push(DocumentLink {
                    line_number: line_counter,
                    label: current_label.trim().to_string(),
                    url: current_url.clone(),
                });
                in_link = false;
            }
            _ => {}
        }
    }

    links
}
```

Add methods to `App`:

```rust
    pub fn follow_current_line_link(&mut self) {
        let current_line = (self.scroll_offset as usize) + 1;
        let matching_link = self
            .links
            .iter()
            .find(|l| l.line_number == current_line || l.line_number.abs_diff(current_line) <= 2)
            .cloned();

        if let Some(link) = matching_link {
            if link.url.starts_with('#') {
                let target = link.url.trim_start_matches('#').to_lowercase();
                let heading_match = self
                    .headings
                    .iter()
                    .find(|h| h.title.to_lowercase().replace(' ', "-").contains(&target) || h.title.to_lowercase().contains(&target));

                if let Some(h) = heading_match {
                    self.scroll_offset = h.start_line.saturating_sub(1) as u16;
                    self.status_message = Some(format!("Jumped to section: {}", h.title));
                } else {
                    self.status_message = Some(format!("Section not found: {}", link.url));
                }
            } else if std::path::Path::new(&link.url).exists() {
                if let Ok(new_content) = crate::core::io::read_markdown_file_safe(std::path::Path::new(&link.url)) {
                    self.load_new_document(&new_content, &link.url);
                } else {
                    self.status_message = Some(format!("Failed to read file: {}", link.url));
                }
            }
        }
    }

    pub fn load_new_document(&mut self, content: &str, title: &str) {
        self.history_stack.push((self.raw_text.clone(), title.to_string(), self.scroll_offset));
        let outline = extract_outline(content);
        let stats = calculate_stats(content);
        let cleaned = clean_markdown(content);
        let flat_headings = flatten_headings(&outline.headings);
        let links = extract_document_links(content);

        self.raw_text = content.to_string();
        self.cleaned_text = cleaned;
        self.scroll_offset = 0;
        self.headings = flat_headings;
        self.links = links;
        self.estimated_tokens = stats.estimated_tokens;
        self.status_message = Some(format!("Loaded document: {}", title));
    }

    pub fn backtrack_history(&mut self) {
        if let Some((prev_text, title, prev_scroll)) = self.history_stack.pop() {
            let outline = extract_outline(&prev_text);
            let stats = calculate_stats(&prev_text);
            let cleaned = clean_markdown(&prev_text);
            let flat_headings = flatten_headings(&outline.headings);
            let links = extract_document_links(&prev_text);

            self.raw_text = prev_text;
            self.cleaned_text = cleaned;
            self.scroll_offset = prev_scroll;
            self.headings = flat_headings;
            self.links = links;
            self.estimated_tokens = stats.estimated_tokens;
            self.status_message = Some(format!("Returned to: {}", title));
        } else {
            self.status_message = Some("Already at root document.".to_string());
        }
    }
```

Wire `KeyCode::Enter` and `KeyCode::Backspace` in `App::handle_key`:
- `KeyCode::Enter` $\rightarrow$ `self.follow_current_line_link();`
- `KeyCode::Backspace` $\rightarrow$ `self.backtrack_history();`

- [ ] **Step 4: Run full test suite and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS (0 errors, 0 clippy warnings)

- [ ] **Step 5: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat: implement interactive link jumping, section anchor scroll, and file history in TUI"
```
