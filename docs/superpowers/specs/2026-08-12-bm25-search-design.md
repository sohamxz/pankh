# Design Specification: BM25 Section Relevance Ranking Engine

## Summary
Upgrade Pankh's multi-document search engine (`src/core/search.rs`) from flat substring matching to an **Okapi BM25 Section Relevance Ranking Engine** with heading level multipliers ($H1 \dots H6$).

---

## 1. Mathematical Scoring Formula

### Okapi BM25 Formula
For a section document $D$ and search query $Q = \{q_1, q_2, \dots, q_n\}$:

$$\text{BM25}(D, Q) = \sum_{i=1}^n \text{IDF}(q_i) \cdot \frac{f(q_i, D) \cdot (k_1 + 1)}{f(q_i, D) + k_1 \cdot \left(1 - b + b \cdot \frac{|D|}{\text{avgdl}}\right)}$$

Where:
- $k_1 = 1.2$ (term frequency saturation parameter)
- $b = 0.75$ (length normalization parameter)
- $|D|$ = token word count of section $D$
- $\text{avgdl}$ = average section token word count across all scanned document sections
- $f(q_i, D)$ = frequency of query term $q_i$ in section $D$
- $\text{IDF}(q_i) = \ln\left(1 + \frac{N - n(q_i) + 0.5}{n(q_i) + 0.5}\right)$ where $N$ is total section count and $n(q_i)$ is count of sections containing $q_i$.

### Heading Title Multiplier ($H_w$)
If a query term $q_i$ matches inside the section's Heading Title, its contribution is multiplied by:
- $H1$: $3.0\times$
- $H2$: $2.5\times$
- $H3$: $2.0\times$
- $H4 \dots H6$: $1.5\times$
- Root Document / Body: $1.0\times$

---

## 2. Component & Interface Changes

### Data Structures (`src/core/search.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchHit {
    pub file_path: String,
    pub heading_path: String,
    pub line_number: usize,
    pub line_snippet: String,
    pub section_tokens: usize,
    pub score: f64, // BM25 relevance score
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultiDocSearchResult {
    pub query: String,
    pub total_hits: usize,
    pub files_searched: usize,
    pub hits: Vec<SearchHit>,
}
```

### Execution Flow
1. **Document & Section Parsing:** Reads input markdown documents, extracts outline sections and line snippets.
2. **Corpus Statistics Calculation:** Calculates total section count $N$ and average section length $\text{avgdl}$.
3. **BM25 Scoring:** Scores each section hit using term frequency, document frequency IDF, section length normalization, and heading multipliers.
4. **Ranking:** Sorts hits in descending order of `score`.
5. **Output Formatting:** Displays score in CLI output (e.g. `[Score: 4.82]`) and outputs `score` field in `--json` mode.

---

## 3. Verification Plan

### Automated Unit Tests
- `test_bm25_heading_multiplier_boost`: Verifies that a section with query in $H1/H2$ ranks higher than a section with query only in body text.
- `test_bm25_length_normalization`: Verifies that concise sections matching query score higher than bloated sections.
- `test_bm25_multi_term_idf`: Verifies multi-word query term IDF scoring.
