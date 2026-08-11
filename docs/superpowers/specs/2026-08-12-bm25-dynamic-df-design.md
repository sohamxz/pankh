# Design Specification: Dynamic BM25 Document Frequency Scoring

## Summary
Refine the Okapi BM25 relevance scoring formula in `src/core/search.rs` to compute dynamic corpus Document Frequency $df(t)$ per term across all search hits, improving search ranking accuracy when querying across large multi-document repositories.

---

## 1. Dynamic DF Algorithm

### Formula Update (`src/core/search.rs`)
1. **Pass 1 (Hit Collection):** Collect `raw_hits` matching any query term.
2. **Pass 2 (Corpus Term Frequency Aggregation):**
   - For each query term $t$, compute $df(t)$ = count of `raw_hits` where line or heading title contains $t$.
3. **Pass 3 (BM25 IDF Scoring):**
   $$\text{IDF}(t) = \ln\left( \frac{N - df(t) + 0.5}{df(t) + 0.5} + 1 \right)$$
   where $N = \text{raw\_hits.len()}$.

---

## 2. Verification Plan

### Automated Unit Tests
- `test_bm25_dynamic_df_scoring`: Verifies that common terms appearing in many hits have lower IDF weight than rare terms appearing in a single hit.
