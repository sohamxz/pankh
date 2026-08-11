# Pankh Robustness & Depth Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform Pankh into a production-grade, highly robust Markdown engine with AST-level event stream cleaning, rich TUI rendering with Vim keybindings, heading-aware token chunking, and expanded MCP tools/resources.

**Architecture:** Modular Rust binary using `pulldown-cmark` for AST event stream transformation, `ratatui`/`syntect` for rich TUI rendering, custom token chunker module, and async `tokio` stdio MCP server.

**Tech Stack:** Rust 2021, `pulldown-cmark` (v0.11), `ratatui` (v0.28), `crossterm` (v0.28), `syntect` (v5), `serde`/`serde_json`, `tokio`, `arboard`.

## Global Constraints

- **Language & Edition:** Rust 2021 Edition.
- **Code Quality:** Zero warnings on `cargo clippy`. All public functions documented.
- **Zero Unsafe:** `#![forbid(unsafe_code)]` across core modules.
- **Verification:** Every task verified with unit/integration tests (`cargo test`).

---

### Task 1: AST Event Stream Transformer & Noise Stripper (`src/core/agent.rs` & `src/core/parser.rs`)

**Files:**
- Modify: `src/core/parser.rs`
- Modify: `src/core/agent.rs`
- Test: `src/core/agent.rs` (unit tests)

**Interfaces:**
- Consumes: Raw Markdown `&str`.
- Produces:
  - `clean_markdown(input: &str) -> String`
  - `clean_markdown_with_report(input: &str) -> CleanerReport`

- [ ] **Step 1: Write failing unit tests for frontmatter, multi-line badge stacks, and URL tracking params in src/core/agent.rs**

```rust
#[test]
fn test_frontmatter_stripping() {
    let raw = "---\ntitle: Test\nauthor: Me\n---\n\n# Main Title\nBody text";
    let cleaned = clean_markdown(raw);
    assert!(!cleaned.contains("author: Me"));
    assert!(cleaned.contains("Main Title"));
}

#[test]
fn test_multiline_badge_link_stripping() {
    let raw = "# Header\n\n[![Build Status]\n(https://img.shields.io/badge.svg)]\n(https://example.com/build)\n\nContent";
    let cleaned = clean_markdown(raw);
    assert!(!cleaned.contains("shields.io"));
    assert!(cleaned.contains("Content"));
}

#[test]
fn test_url_tracking_param_stripping() {
    let raw = "[Click Here](https://example.com/page?utm_source=twitter&ref=abc)";
    let cleaned = clean_markdown(raw);
    assert!(cleaned.contains("https://example.com/page"));
    assert!(!cleaned.contains("utm_source"));
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test core::agent::tests::test_frontmatter_stripping`
Expected: FAIL.

- [ ] **Step 3: Implement AST Event Transformer in src/core/agent.rs**

Implement frontmatter parsing, badge tag stack state tracker, HTML comment/SVG filter, URL tracking query param stripper, and token metrics calculation.

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test core::agent`
Expected: PASS.

- [ ] **Step 5: Commit Task 1**

```bash
git add src/core/
git commit -m "feat: implement pure AST Event Stream Transformer for token optimization"
```

---

### Task 2: Intelligent Token Budget Chunker (`src/core/chunker.rs`)

**Files:**
- Create: `src/core/chunker.rs`
- Modify: `src/core/mod.rs`
- Test: `src/core/chunker.rs` (unit tests)

**Interfaces:**
- Consumes: Raw Markdown `&str`, `max_tokens: usize`.
- Produces: `chunk_markdown(input: &str, max_tokens: usize) -> Vec<MarkdownChunk>`

- [ ] **Step 1: Write failing unit test in src/core/chunker.rs**

```rust
#[test]
fn test_token_chunking() {
    let raw = "# H1\n\nSection 1 text.\n\n## H2\n\nSection 2 text.\n\n### H3\n\nSection 3 text.";
    let chunks = chunk_markdown(raw, 20);
    assert!(chunks.len() > 1);
    for chunk in &chunks {
        assert!(chunk.token_count <= 25);
    }
}
```

- [ ] **Step 2: Implement token chunking logic in src/core/chunker.rs**

Iterate document sections by heading levels. Group adjacent sections into a single chunk until adding the next section exceeds `max_tokens`.

- [ ] **Step 3: Run tests to verify pass**

Run: `cargo test core::chunker`
Expected: PASS.

- [ ] **Step 4: Commit Task 2**

```bash
git add src/core/chunker.rs src/core/mod.rs
git commit -m "feat: implement heading-aware token budget document chunker"
```

---

### Task 3: Rich TUI Renderer & Vim Motion Navigation (`src/tui/render.rs`, `src/tui/app.rs`, `src/tui/ui.rs`)

**Files:**
- Create: `src/tui/render.rs`
- Modify: `src/tui/app.rs`
- Modify: `src/tui/ui.rs`
- Test: `src/tui/app.rs` (unit tests)

**Interfaces:**
- Consumes: App state, Markdown AST events.
- Produces: Styled Ratatui lines with bold headers, blockquotes (`│`), bullet list icons, syntax highlighting, and `n`/`N` search navigation.

- [ ] **Step 1: Write unit tests for Vim motions and search navigation in src/tui/app.rs**

```rust
#[test]
fn test_tui_vim_motions_and_search() {
    let mut app = App::new("# H1\n\nFirst line\n\nSecond line\n\nThird line");
    app.handle_key(crossterm::event::KeyCode::Char('G'), crossterm::event::KeyModifiers::NONE);
    assert!(app.scroll_offset > 0);
    app.handle_key(crossterm::event::KeyCode::Char('g'), crossterm::event::KeyModifiers::NONE);
    assert_eq!(app.scroll_offset, 0);
}
```

- [ ] **Step 2: Implement Rich TUI Renderer in src/tui/render.rs and wire into ui.rs**

Implement AST line renderer formatting header levels with distinct colors, blockquotes with left border `│`, list bullet symbols, and search match highlight positions. Implement `n`/`N` search match jumping in `app.rs`.

- [ ] **Step 3: Run tests to verify pass**

Run: `cargo test tui::app`
Expected: PASS.

- [ ] **Step 4: Commit Task 3**

```bash
git add src/tui/
git commit -m "feat: implement rich Markdown TUI renderer and Vim motion navigation"
```

---

### Task 4: Expanded MCP Server (Tools, Resources & Section Search)

**Files:**
- Modify: `src/mcp/server.rs`
- Test: `tests/mcp_test.rs`

**Interfaces:**
- Consumes: Stdio JSON-RPC 2.0 requests.
- Produces: Responses for `tools/list`, `tools/call`, `resources/list`, `resources/read`.

- [ ] **Step 1: Write unit test for new MCP tools & resources in tests/mcp_test.rs**

```rust
#[tokio::test]
async fn test_mcp_search_sections_tool() {
    let req = r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"search_markdown_sections","arguments":{"path":"tests/sample.md","query":"Installation"}}}"#;
    let res = handle_jsonrpc_message(req).await.unwrap().unwrap();
    assert!(res.contains("Installation"));
}
```

- [ ] **Step 2: Implement search_markdown_sections, chunk_markdown, estimate_tokens, and resources/list in src/mcp/server.rs**

- [ ] **Step 3: Run tests to verify pass**

Run: `cargo test --test mcp_test`
Expected: PASS.

- [ ] **Step 4: Commit Task 4**

```bash
git add src/mcp/ tests/mcp_test.rs
git commit -m "feat: expand MCP server with section search, token chunking, and resource handlers"
```

---

### Task 5: Multi-File CLI Pipeline & Final Verification

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/cli_test.rs`
- Modify: `README.md`

- [ ] **Step 1: Add --max-tokens and multi-file CLI support in src/main.rs**
- [ ] **Step 2: Run full test suite and clippy linting**

Run: `cargo test`
Run: `cargo clippy -- -D warnings`
Expected: PASS with 0 failures and 0 warnings.

- [ ] **Step 3: Commit Task 5**

```bash
git add src/main.rs tests/cli_test.rs README.md
git commit -m "feat: add --max-tokens CLI flag, multi-file support, and finalize documentation"
```
