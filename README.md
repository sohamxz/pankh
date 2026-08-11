# Pankh (🪶) v0.1.1
> **Featherweight Markdown Reader for Humans & AI Agents**

Pankh (*Feather* in Hindi) is an ultra-fast, multi-modal Rust CLI binary, Terminal UI (TUI), Native Desktop GUI, and Model Context Protocol (MCP) server engineered for developers and AI coding agents.

---

## Features

- **Optional Native Desktop GUI (`pankh -g [file]`):**
  - **Zero Bundle Overhead (`wry` + `tao`):** Reuses your OS's built-in webview engine (WebView2 on Windows, WKWebView on macOS, WebKitGTK on Linux) — adding **0MB browser engine bundle size** unlike heavy Electron apps.
  - **Native OS File Picker (`rfd`):** Click **"📂 Open File"** or press **`Ctrl+O`** to open native Windows File Explorer / macOS Finder file selector.
  - **Glassmorphic Navigation Bar:** Real-time token count (`⚡ Tokens`), word count (`📝 Words`), theme selector dropdown, and collapsible Table of Contents (TOC) sidebar.

- **Rich Interactive TUI Reader (Human Mode):**
  - Minimalist dark terminal UI powered by `ratatui` with header level hierarchy styling ($H1 \dots H6$).
  - **Interactive Link Navigation:** Press `Enter` on any line with Markdown links to jump to section anchors (`#heading`) or load relative Markdown files (`doc.md`). Press `Backspace` to navigate back through file history.
  - **Fuzzy File Finder (`Ctrl+P` / `f`):** Instant repository-wide Markdown fuzzy file search modal.
  - **Theme Switcher (`t`):** Toggle between Ocean Dark 🌙, Dracula 🧛, Gruvbox 🌲, and Clean Light ☀️ themes with 0ms scrolling latency.
  - **Code Snippet Copy (`y`):** One-key code block copying directly to system clipboard.
  - **Live File Watcher (`pankh README.md --watch`):** Live-reloads content on save while preserving scroll offset and Table of Contents state.
  - Formatted Unicode grid tables (`┌──────┬──────┐`).
  - Interactive task list checkboxes (`[✓]` / `[ ]`).
  - Nested list bullet symbols (`*`, `-`, `+`).
  - `syntect` syntax highlighting for fenced code blocks.
  - Vim navigation shortcuts (`g`/`G` top/bottom, `Ctrl+u`/`Ctrl+d` half page, `n`/`N` next/prev search result match).

- **Real-Time Auto-Indexing MCP Daemon:**
  - `pankh --mcp` spawns a real-time background file watcher thread that updates shared `Arc<RwLock<SearchIndex>>` state on file additions, edits, or removals.
  - Emits JSON-RPC `notifications/resources/updated` notifications to connected LLM clients (Cursor, Claude Desktop, Antigravity) for live context sync.

- **Levenshtein Fuzzy BM25 Relevance Search Engine:**
  - Typo-tolerant BM25 search matching queries with misspellings (e.g. `pankh -S "instalation"` matches `"Installation"`).
  - Exact hits receive full Okapi score ($1.0\times$), while fuzzy matches receive similarity-weighted Okapi scores ($\le 2$ edit distance).
  - Advanced boolean AST query parser supporting field filters (`path:docs/`, `ext:md`, `lang:rs`, `dir:tests`), exact phrase matching (`"phrase"`), and negated exclusions (`-deprecated`).

- **Incremental Binary Search Indexing (`pankh --build-index`):**
  - Builds `.pankh_index.bin` for instant sub-5ms BM25 search across 10,000+ file monorepos.
  - Incremental update system compares modification timestamps (`mtime_secs`) and file sizes, re-indexing only new or modified files.

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

---

## Installation

Ensure you have Rust installed (1.78+):

```bash
git clone https://github.com/sohamxz/pankh.git
cd pankh
cargo install --path .
```

Alternatively, download single pre-compiled executables for Windows (`pankh-windows.exe`), macOS (`pankh-macos`), or Linux (`pankh-linux`) from [GitHub Releases](https://github.com/sohamxz/pankh/releases).

---

## Usage

### 1. Optional Native Desktop GUI

```bash
# Open README.md in Native Desktop GUI
pankh -g README.md

# Open Desktop GUI to pick a file
pankh -g
```

### 2. Human Interactive TUI Reader

```bash
# Open document in TUI
pankh README.md

# Open document in TUI with live file watcher live-reloading on save
pankh README.md --watch
```

### 3. TUI Keybindings

| Key | Action |
| :--- | :--- |
| **`j` / `k`** | Scroll Down / Up |
| **`g` / `G`** | Jump to Top / Bottom |
| **`Ctrl+d` / `Ctrl+u`** | Scroll Half Page Down / Up |
| **`/`** | Focus Search Input |
| **`n` / `N`** | Next / Previous Search Match |
| **`t`** | Cycle Theme (Ocean Dark, Dracula, Gruvbox, Clean Light) |
| **`Tab` / `b`** | Toggle Table of Contents (TOC) Sidebar |
| **`Ctrl+P` / `f`** | Open Repository Fuzzy File Finder |
| **`y`** | Copy Focused Code Block to Clipboard |
| **`a`** | Copy Clean Token-Thrifty Text to Clipboard |
| **`Enter`** | Follow Markdown Link / Jump to Section Anchor |
| **`Backspace`** | Backtrack File History |
| **`Esc`** | Clear Search / Close Overlay / Quit |
| **`q`** | Quit Pankh |

---

### 4. Search & Agent Commands

```bash
# Build pre-computed binary search index for instant sub-5ms search across monorepos
pankh docs/ --build-index

# BM25 Relevance Search with Levenshtein fuzzy matching and field filters
pankh -S 'path:docs/ "installation guide" -deprecated' docs/ [--json]

# Auto-regenerate llms.txt & llms-full.txt whenever documentation changes
pankh --llms-txt docs/ --watch

# Display stats with LLM input cost estimation
pankh README.md --stats [--price-per-m 2.50]

# Output token-thrifty clean markdown for AI agents
pankh README.md --agent
```

---

## Architecture

Pankh is modularly structured into core submodules:

- `pankh::core::agent`: AST Event Stream Cleaner & Diff Generator.
- `pankh::core::chunker`: Heading-Aware Token Budget Document Chunker.
- `pankh::core::search`: Hyper-Parallel Levenshtein Fuzzy BM25 Relevance Search Engine (`rayon`).
- `pankh::core::index`: Pre-computed Search Indexing Engine (`.pankh_index.bin`) & Trigram Posting Storage.
- `pankh::core::query`: Advanced Query AST Parser & Field Filter Matcher (`path:`, `ext:`, `lang:`, `dir:`).
- `pankh::core::llmstxt`: `llms.txt` & `llms-full.txt` AI Documentation Generator.
- `pankh::core::pricing`: Dynamic Future-Proof LLM Cost Estimator.
- `pankh::core::watcher`: Cross-Platform File & Directory Watcher (`notify`).
- `pankh::core::io`: Safe File Reader (50MB cap, null byte binary detector, lossy UTF-8).
- `pankh::gui`: Ultra-Lightweight Native Desktop GUI Engine (`wry` + `tao` + `rfd`).
- `pankh::tui`: Ratatui Terminal Interface with Rich AST Markdown Renderer, Render Line Caching, Interactive Link Navigation, Vim Motions, and Panic Hook.
- `pankh::mcp`: Stdio JSON-RPC 2.0 MCP Protocol Server & Real-time Auto-Indexing Daemon.

---

## License

Dual-licensed under MIT or Apache 2.0.
