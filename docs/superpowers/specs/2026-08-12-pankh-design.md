# Design Specification: Pankh (Featherweight Markdown Reader & AI Context Engine)

**Date:** 2026-08-12  
**Status:** Approved (Comprehensive Robustness & Depth Upgrade)  
**Language:** Rust  

---

## 1. Overview & Vision

**Pankh** (*Feather* in Hindi) is an ultra-fast, single-binary Rust tool designed to bridge the gap between human markdown reading and AI agent context ingestion.

* **For Humans:** Provides a distraction-free, aesthetically pleasing Terminal User Interface (TUI) with dark palette styling, rich Markdown formatting (header hierarchies, blockquotes, lists, syntax highlighting), Table-of-Contents navigation, search with `n`/`N` match jumping, and Vim motion keybindings.
* **For AI Agents:** Operates as a pure AST-level event transformer (stripping visual badges, tracking images, HTML comments, `<svg>`/`<picture>` elements, long link tracking parameters, YAML frontmatter, and redundant line breaks) and as a full-featured Model Context Protocol (MCP) server.

---

## 2. System Architecture & Components

```
                                 ┌──────────────────┐
                                 │   pankh binary   │
                                 └────────┬─────────┘
                                          │
            ┌─────────────────────────────┼─────────────────────────────┐
            │                             │                             │
    [1. Human Rich TUI]            [2. Agent CLI]                [3. MCP Server]
       pankh doc.md                 pankh doc -a                  pankh --mcp
  (Vim keys, Rich MD,             (--max-tokens,              (Tools + Resources +
   TOC drawer, Search)             --outline, --code)            Section Search)
            │                             │                             │
            ▼                             ▼                             ▼
      ┌──────────┐                  ┌──────────┐                  ┌──────────┐
      │ Ratatui  │                  │ Token    │                  │ Stdio    │
      │ TUI UI   │                  │ Stripper │                  │ JSON-RPC │
      └────┬─────┘                  └────┬─────┘                  └────┬─────┘
           │                             │                             │
           └─────────────────────────────┼─────────────────────────────┘
                                         │
                               ┌─────────▼──────────┐
                               │  Core AST Engine   │
                               │ (pulldown-cmark)   │
                               └────────────────────┘
```

---

## 3. Core Engine & Subsystems

### A. AST Event Stream Transformer (`src/core/agent.rs` & `src/core/parser.rs`)
1. **Frontmatter Stripper:** Detects and strips YAML (`---`) or TOML (`+++`) metadata blocks.
2. **Badge Tag Stack Filter:** Tracks nested link/image AST events (`Event::Start(Tag::Link)`, `Event::Start(Tag::Image)`). Suppresses badge links matching badge domains (`shields.io`, `badge.fury`, `codecov`, `workflows`, `circleci`, `sonarcloud`, `crates.io/badges`) and reference link definitions.
3. **HTML & Comment Suppressor:** Strips `Event::Html` and `Event::InlineHtml` matching `<!-- comment -->`, `<svg>...</svg>`, `<picture>...</picture>`, `<nav>...</nav>`, and tracking pixels.
4. **URL Query Parameter Stripper:** Strips tracking parameters (`?utm_source=...`, `?ref=...`) from link destinations while retaining human anchor text.
5. **Token Reduction Metrics:** Outputs `CleanerReport` struct with raw token count, cleaned token count, tokens saved, and percentage reduction.

### B. Intelligent Token Chunker (`src/core/chunker.rs`)
- `chunk_markdown(input: &str, max_tokens: usize) -> Vec<MarkdownChunk>`
- Slices documents at heading boundaries so that each chunk stays strictly within `max_tokens` budget without splitting mid-sentence or breaking code blocks.

### C. Rich TUI Markdown Renderer (`src/tui/render.rs` & `src/tui/app.rs`)
- **Rich Elements:** Formatted headers ($H1 \dots H6$), styled blockquotes (`│`), bullet lists (`•`, `⁃`), code blocks with syntax highlighting (`syntect`), task checkboxes (`[✓]`, `[ ]`), and table grid borders.
- **Vim & Navigation Keys:**
  - `j` / `k` or `Down` / `Up`: Scroll line by line.
  - `g` / `G`: Jump to top / bottom of document.
  - `Ctrl+u` / `Ctrl+d`: Scroll half page up / down.
  - `Tab` or `b`: Toggle TOC sidebar drawer.
  - `/`: Search prompt.
  - `n` / `N`: Jump to next / previous search match.
  - `a`: Copy clean token-optimized text to system clipboard.
  - `q` / `Esc`: Exit app.

### D. Extended MCP Protocol Handler (`src/mcp/`)
- MCP Tools:
  1. `read_clean_markdown(path: string)`
  2. `get_markdown_outline(path: string)`
  3. `read_markdown_section(path: string, heading: string)`
  4. `extract_code_blocks(path: string, lang?: string)`
  5. `search_markdown_sections(path: string, query: string)`
  6. `chunk_markdown(path: string, max_tokens: number)`
  7. `estimate_tokens(path: string)`
- MCP Resources (`resources/list`, `resources/read`):
  - Exposes local markdown file subscriptions via `file:///` URIs.

---

## 4. Verification & Testing Strategy
* **Unit Tests:**
  - AST badge stack & multi-line link stripping.
  - YAML frontmatter stripping.
  - Heading outline tree & line range calculations.
  - Token chunking under specified token limits.
* **Integration Tests:**
  - CLI flags: `--agent`, `--outline`, `--code`, `--stats`, `--max-tokens`, `--json`.
  - Stdio JSON-RPC MCP server tool calls and resource reads.
