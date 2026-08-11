# Pankh (🪶)
> **Featherweight Markdown Reader for Humans & AI Agents**

Pankh (*Feather* in Hindi) is an ultra-fast, dual-modality Rust CLI binary & Model Context Protocol (MCP) server engineered for developers and AI coding agents.

---

## Features

- **Rich Interactive TUI Reader (Human Mode):**
  - Minimalist dark terminal UI powered by `ratatui` with header level hierarchy styling ($H1 \dots H6$).
  - **Interactive Link Navigation:** Press `Enter` on any line with Markdown links to jump to section anchors (`#heading`) or load relative Markdown files (`doc.md`). Press `Backspace` to navigate back through file history.
  - **Live File Watcher (`pankh README.md --watch`):** Live-reloads content on save while preserving scroll offset and Table of Contents state.
  - Formatted Unicode grid tables (`┌──────┬──────┐`).
  - Interactive task list checkboxes (`[✓]` / `[ ]`).
  - Nested list bullet symbols (`*`, `-`, `+`).
  - `syntect` syntax highlighting for fenced code blocks.
  - Vim navigation shortcuts (`g`/`G` top/bottom, `Ctrl+u`/`Ctrl+d` half page, `n`/`N` next/prev search result match).
  - Native terminal mouse wheel scrolling and Table of Contents (TOC) sidebar (`Tab` / `b`).
  - Quick clean text clipboard copy (`a`).

- **Real-Time Auto-Indexing MCP Daemon:**
  - `pankh mcp` spawns a real-time background file watcher thread that updates shared `Arc<RwLock<SearchIndex>>` state on file additions, edits, or removals.
  - Emits JSON-RPC `notifications/resources/updated` notifications to connected LLM clients (Cursor, Claude Desktop, Antigravity) for live context sync.

- **Advanced Query Syntax & Trigram Search Engine:**
  - Boolean AST parser supporting field filters (`path:docs/`, `ext:md`, `lang:rs`, `dir:tests`), exact phrase matching (`"phrase"`), and negated exclusions (`-deprecated`).
  - Integrated character trigram posting index (`trigram_posting`) with sub-2ms Jaccard similarity fallback for typo-tolerant fuzzy matching.

- **Pre-computed Binary Search Indexing (`pankh --build-index`):**
  - Build `.pankh_index.bin` for instant sub-5ms BM25 search across 10,000+ file monorepos.
  - Automatic detection loads the binary index instantly when running `pankh -S "query"`.

- **Hyper-Parallel BM25 Relevance Search Engine (`rayon`):**
  - Multi-threaded parallel search across single files, multiple files, or entire directory trees (`pankh -S "query" docs/`).
  - Powered by Okapi BM25 relevance scoring ($k_1=1.2, b=0.75$) with Heading Title multipliers ($H1=3.0\times, H2=2.5\times, H3=2.0\times$) and dynamic corpus $df(t)$ calculation.
  - Maps matches to file path, line number, heading context, snippet, and section token count.

- **LLM Cost Estimator:**
  - Calculates estimated input token costs and dollar savings across `--stats` and `--diff-clean`.
  - Tier-based defaults (Frontier \$5.00/1M, Production \$0.50/1M, Budget \$0.05/1M, Local \$0.00/1M).
  - Configurable via `--price-per-m <RATE>` CLI flag, `~/.config/pankh/pricing.json`, or `PANKH_PRICING_FILE`.

- **`llms.txt` Standard Generator:**
  - `pankh --llms-txt [DIR]` generates standard `llms.txt` (project documentation index) and `llms-full.txt` (concatenated AST-cleaned token-thrifty payload) for AI agents.
  - Combined with `--watch` (`pankh --llms-txt docs/ --watch`), auto-regenerates `llms.txt` and `llms-full.txt` whenever any documentation file is edited or saved.

- **AST-Level Token Optimizer & Chunker (Agent Mode):**
  - Pure AST Event Stream Transformer (`pulldown-cmark`) strips visual badge links (`img.shields.io`), HTML comments (`<!-- ... -->`), raw SVGs, and URL tracking parameters (`utm_*`, `ref=`, `spm=`).
  - Reduces LLM prompt token consumption by 20% to 40%.
  - `--max-tokens <N>` heading-aware budget chunking slices large documents at section boundaries without breaking code blocks or paragraphs mid-sentence.
  - Multi-file pipeline support (`pankh doc1.md doc2.md --agent`).

- **Model Context Protocol (MCP) Server:**
  - Stdio JSON-RPC 2.0 MCP server (`pankh --mcp`) compatible with Claude Desktop, Cursor, and Antigravity.
  - **7 MCP Tools:** `read_clean_markdown`, `get_markdown_outline`, `read_markdown_section`, `extract_code_blocks`, `search_markdown_sections`, `chunk_markdown`, `estimate_tokens`.
  - **MCP Resources:** Access local Markdown files dynamically via `file:///` URIs.

---

## Installation

Ensure you have Rust installed (1.78+):

```bash
git clone https://github.com/sohamxz/pankh.git
cd pankh
cargo install --path .
```

---

## Usage

### 1. Human Interactive TUI Reader

```bash
# Open document in TUI
pankh README.md

# Open document in TUI with live file watcher live-reloading on save
pankh README.md --watch
```

---

### 2. Instant Pre-computed Search & Advanced Query Syntax

```bash
# Build pre-computed binary search index for instant sub-5ms search across monorepos
pankh docs/ --build-index

# BM25 Relevance Search with field filters and exact phrase matching
pankh -S 'path:docs/ "installation guide" -deprecated' docs/ [--json]

# Auto-regenerate llms.txt & llms-full.txt whenever documentation changes
pankh --llms-txt docs/ --watch

# Display stats with LLM input cost estimation
pankh README.md --stats [--price-per-m 2.50]

# Output token-thrifty clean markdown
pankh README.md --agent
```

---

## Architecture

Pankh is modularly structured into core submodules:

- `pankh::core::agent`: AST Event Stream Cleaner & Diff Generator.
- `pankh::core::chunker`: Heading-Aware Token Budget Document Chunker.
- `pankh::core::search`: Hyper-Parallel BM25 Relevance Search Engine (`rayon`).
- `pankh::core::index`: Pre-computed Search Indexing Engine (`.pankh_index.bin`) & Trigram Posting Storage.
- `pankh::core::query`: Advanced Query AST Parser & Field Filter Matcher (`path:`, `ext:`, `lang:`, `dir:`).
- `pankh::core::llmstxt`: `llms.txt` & `llms-full.txt` AI Documentation Generator.
- `pankh::core::pricing`: Dynamic Future-Proof LLM Cost Estimator.
- `pankh::core::watcher`: Cross-Platform File & Directory Watcher (`notify`).
- `pankh::core::io`: Safe File Reader (50MB cap, null byte binary detector, lossy UTF-8).
- `pankh::tui`: Ratatui Terminal Interface with Rich AST Markdown Renderer, Interactive Link Navigation, Vim Motions, and Panic Hook.
- `pankh::mcp`: Stdio JSON-RPC 2.0 MCP Protocol Server & Real-time Auto-Indexing Daemon.

---

## License

Dual-licensed under MIT or Apache 2.0.
