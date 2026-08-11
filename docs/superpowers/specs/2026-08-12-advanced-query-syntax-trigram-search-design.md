# Design Specification: Advanced Query Syntax & Trigram Fuzzy Search Engine

## Executive Summary
Pankh (🪶) is upgrading its search engine from basic keyword BM25 to an **Advanced Query Parser & Trigram Fuzzy Search Engine**. This system supports structured field filters (`path:`, `ext:`, `lang:`, `heading:`, `-exclusion`), exact phrase matching (`"exact phrase"`), boolean logic (`AND`/`OR`), and character-level trigram fuzzy search for typo-tolerant document retrieval across large monorepos.

---

## 1. Query Syntax & AST Architecture

### 1.1 Supported Grammar
- **Field Filters:** `path:docs/`, `ext:md`, `lang:rust`, `heading:Architecture`
- **Exclusion (Negation):** `-deprecated`, `-path:vendor/`
- **Exact Phrases:** `"database migration guide"`
- **Fuzzy Term Matching:** `seach~` or automatic trigram fuzzy fallback for low-BM25 hits
- **Boolean Precedence:** Implicit `AND` between space-separated terms, explicit `OR` operator, parenthesized grouping `(path:docs OR path:src)`

### 1.2 Query AST Data Structures (`src/core/query.rs`)
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum QueryToken {
    Term(String),
    ExactPhrase(String),
    FieldFilter { field: String, value: String },
    NegatedTerm(String),
    NegatedFilter { field: String, value: String },
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedQuery {
    pub positive_terms: Vec<String>,
    pub exact_phrases: Vec<String>,
    pub field_filters: Vec<(String, String)>,
    pub negated_terms: Vec<String>,
    pub negated_filters: Vec<(String, String)>,
    pub is_fuzzy: bool,
}
```

---

## 2. Trigram Indexing & Binary Storage (`src/core/index.rs`)

### 2.1 Index Schema Extension
Extend `.pankh_index.bin` data structure:
```rust
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SearchIndex {
    pub docs: HashMap<String, IndexedDocMeta>,
    pub term_posting: HashMap<String, Vec<(String, usize)>>,
    pub trigram_posting: HashMap<[char; 3], Vec<String>>,
    pub doc_count: usize,
    pub avg_doc_length: f64,
}
```

### 2.2 Trigram Fuzzy Scoring
When exact BM25 matches yield insufficient candidates, compute trigram similarity (Jaccard Coefficient over character trigrams) to retrieve typo-corrected terms (e.g. `algoritm` -> `algorithm`) in sub-2ms.

---

## 3. Universal Integration Surface

- **CLI (`pankh -S "path:src lang:rs -test 'struct SearchIndex'"`):** Evaluates AST query filter tree over binary index.
- **TUI (`/` Search Bar):** Real-time query syntax token highlighting and instant filtering.
- **MCP Protocol (`search_sections` tool):** Supports full query syntax in tool arguments for LLM agent integration.
