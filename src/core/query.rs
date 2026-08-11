use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ParsedQuery {
    pub raw_query: String,
    pub positive_terms: Vec<String>,
    pub exact_phrases: Vec<String>,
    pub field_filters: Vec<(String, String)>,
    pub negated_terms: Vec<String>,
    pub negated_filters: Vec<(String, String)>,
}

impl ParsedQuery {
    pub fn is_empty(&self) -> bool {
        self.positive_terms.is_empty()
            && self.exact_phrases.is_empty()
            && self.field_filters.is_empty()
            && self.negated_terms.is_empty()
            && self.negated_filters.is_empty()
    }

    /// Evaluates whether a file path and text content matches field filters & negated exclusions
    pub fn matches_filters(&self, file_path_str: &str, content: &str) -> bool {
        let path_lower = file_path_str.to_lowercase();
        let content_lower = content.to_lowercase();

        // 1. Evaluate field filters (e.g. path:docs/ lang:rs ext:md)
        for (field, val) in &self.field_filters {
            let val_lower = val.to_lowercase();
            match field.as_str() {
                "path" | "dir" => {
                    if !path_lower.contains(&val_lower) {
                        return false;
                    }
                }
                "ext" => {
                    if !path_lower.ends_with(&format!(".{}", val_lower.trim_start_matches('.'))) {
                        return false;
                    }
                }
                "lang" => {
                    if !content_lower.contains(&format!("```{}", val_lower)) {
                        return false;
                    }
                }
                _ => {
                    if !path_lower.contains(&val_lower) && !content_lower.contains(&val_lower) {
                        return false;
                    }
                }
            }
        }

        // 2. Evaluate negated filters & terms (-deprecated -path:vendor)
        for (field, val) in &self.negated_filters {
            let val_lower = val.to_lowercase();
            match field.as_str() {
                "path" | "dir" => {
                    if path_lower.contains(&val_lower) {
                        return false;
                    }
                }
                _ => {
                    if path_lower.contains(&val_lower) || content_lower.contains(&val_lower) {
                        return false;
                    }
                }
            }
        }

        for neg in &self.negated_terms {
            let neg_lower = neg.to_lowercase();
            if content_lower.contains(&neg_lower) || path_lower.contains(&neg_lower) {
                return false;
            }
        }

        // 3. Evaluate exact phrases ("database migration")
        for phrase in &self.exact_phrases {
            let phrase_lower = phrase.to_lowercase();
            if !content_lower.contains(&phrase_lower) && !path_lower.contains(&phrase_lower) {
                return false;
            }
        }

        true
    }
}

/// Parses a query string into a structured ParsedQuery AST
pub fn parse_query(raw_query: &str) -> ParsedQuery {
    let mut parsed = ParsedQuery {
        raw_query: raw_query.to_string(),
        ..Default::default()
    };

    let chars = raw_query.chars();
    let mut current_token = String::new();
    let mut in_quote = false;

    for ch in chars {
        if ch == '"' {
            if in_quote {
                if !current_token.trim().is_empty() {
                    parsed.exact_phrases.push(current_token.trim().to_string());
                }
                current_token.clear();
                in_quote = false;
            } else {
                if !current_token.trim().is_empty() {
                    process_token(current_token.trim(), &mut parsed);
                    current_token.clear();
                }
                in_quote = true;
            }
        } else if ch.is_whitespace() && !in_quote {
            if !current_token.trim().is_empty() {
                process_token(current_token.trim(), &mut parsed);
                current_token.clear();
            }
        } else {
            current_token.push(ch);
        }
    }

    if !current_token.trim().is_empty() {
        if in_quote {
            parsed.exact_phrases.push(current_token.trim().to_string());
        } else {
            process_token(current_token.trim(), &mut parsed);
        }
    }

    parsed
}

fn process_token(token: &str, parsed: &mut ParsedQuery) {
    if let Some(rest) = token.strip_prefix('-') {
        if let Some(pos) = rest.find(':') {
            let field = rest[..pos].to_lowercase();
            let value = rest[pos + 1..].to_string();
            parsed.negated_filters.push((field, value));
        } else {
            let clean = rest
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            if !clean.is_empty() {
                parsed.negated_terms.push(clean);
            }
        }
    } else if let Some(pos) = token.find(':') {
        let field = token[..pos].to_lowercase();
        let value = token[pos + 1..].to_string();
        parsed.field_filters.push((field, value));
    } else {
        let clean = token
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if !clean.is_empty() {
            parsed.positive_terms.push(clean);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_syntax() {
        let q = parse_query("path:docs/ lang:rs -deprecated \"exact phrase\" term");
        assert_eq!(q.field_filters.len(), 2);
        assert_eq!(
            q.field_filters[0],
            ("path".to_string(), "docs/".to_string())
        );
        assert_eq!(q.field_filters[1], ("lang".to_string(), "rs".to_string()));
        assert_eq!(q.negated_terms, vec!["deprecated"]);
        assert_eq!(q.exact_phrases, vec!["exact phrase"]);
        assert_eq!(q.positive_terms, vec!["term"]);
    }

    #[test]
    fn test_query_filter_matching() {
        let q = parse_query("path:docs/ -draft \"api guide\"");
        assert!(q.matches_filters("docs/api.md", "This is an api guide for production."));
        assert!(!q.matches_filters("src/main.rs", "This is an api guide for production."));
        assert!(!q.matches_filters("docs/api.md", "This is a draft api guide."));
    }
}
