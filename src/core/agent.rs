use crate::core::parser::parse_markdown;
use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};
use serde::{Deserialize, Serialize};

/// Maximum recursion depth allowed during heading tree nesting to prevent stack overflow
const MAX_NESTING_DEPTH: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeadingNode {
    pub level: u32,
    pub title: String,
    pub start_line: usize,
    pub end_line: usize,
    pub token_count: usize,
    pub character_count: usize,
    pub children: Vec<HeadingNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutlineTree {
    pub headings: Vec<HeadingNode>,
    pub total_sections: usize,
    pub max_depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeBlock {
    pub language: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentStats {
    pub lines: usize,
    pub words: usize,
    pub characters: usize,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CleanerReport {
    pub cleaned_text: String,
    pub raw_tokens: usize,
    pub cleaned_tokens: usize,
    pub tokens_saved: usize,
    pub reduction_percentage: f64,
}

/// Calculates line count consumed by frontmatter header block
pub fn get_frontmatter_line_count(input: &str) -> usize {
    let leading_blank_lines = input.lines().take_while(|l| l.trim().is_empty()).count();
    let trimmed = input.trim_start();
    if trimmed.starts_with("---") {
        if let Some(rest) = trimmed.strip_prefix("---") {
            if let Some(end_idx) = rest.find("\n---") {
                let header_slice = &trimmed[..3 + end_idx + 4];
                return leading_blank_lines + header_slice.lines().count();
            }
        }
    } else if trimmed.starts_with("+++") {
        if let Some(rest) = trimmed.strip_prefix("+++") {
            if let Some(end_idx) = rest.find("\n+++") {
                let header_slice = &trimmed[..3 + end_idx + 4];
                return leading_blank_lines + header_slice.lines().count();
            }
        }
    }
    0
}

/// Strips YAML (---) or TOML (+++) frontmatter header from input text
pub fn strip_frontmatter(input: &str) -> &str {
    let trimmed = input.trim_start();
    if let Some(rest) = trimmed.strip_prefix("---") {
        if let Some(end_idx) = rest.find("\n---") {
            let body = &rest[end_idx + 4..];
            if let Some(stripped) = body.strip_prefix("\r\n") {
                return stripped;
            } else if let Some(stripped) = body.strip_prefix('\n') {
                return stripped;
            }
            return body;
        }
    } else if let Some(rest) = trimmed.strip_prefix("+++") {
        if let Some(end_idx) = rest.find("\n+++") {
            let body = &rest[end_idx + 4..];
            if let Some(stripped) = body.strip_prefix("\r\n") {
                return stripped;
            } else if let Some(stripped) = body.strip_prefix('\n') {
                return stripped;
            }
            return body;
        }
    }
    input
}

/// Checks if an image or link URL matches known badge domains/patterns
fn is_badge_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.contains("shields.io")
        || lower.contains("badge")
        || lower.contains("workflows/")
        || lower.contains("codecov")
        || lower.contains("circleci")
        || lower.contains("sonarcloud")
        || lower.contains("travis-ci")
        || lower.contains("github.com/actions/workflows")
        || lower.contains("crates.io/badges")
}

/// Strips tracking query parameters from URL string handling anchors (#section) and malformed query strings
fn strip_tracking_params(url: &str) -> String {
    let (url_without_anchor, anchor) = match url.find('#') {
        Some(pos) => (&url[..pos], &url[pos..]),
        None => (url, ""),
    };

    if let Some(pos) = url_without_anchor.find('?') {
        let base = &url_without_anchor[..pos];
        let query = &url_without_anchor[pos + 1..];

        let clean_params: Vec<&str> = query
            .split('&')
            .filter(|param| {
                let trimmed = param.trim();
                if trimmed.is_empty() {
                    return false;
                }
                let lower = trimmed.to_lowercase();
                !lower.starts_with("utm_")
                    && !lower.starts_with("ref=")
                    && !lower.starts_with("spm=")
                    && !lower.starts_with("fbclid=")
                    && !lower.starts_with("gclid=")
            })
            .collect();

        let clean_query = if clean_params.is_empty() {
            base.to_string()
        } else {
            format!("{}?{}", base, clean_params.join("&"))
        };

        format!("{}{}", clean_query, anchor)
    } else {
        url.to_string()
    }
}

/// AST Event Stream Transformer that filters badges, raw HTML comments/SVGs, and url tracking
pub fn clean_markdown(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let content_without_frontmatter = strip_frontmatter(input);
    let parser = parse_markdown(content_without_frontmatter);

    let mut result = String::with_capacity(content_without_frontmatter.len());
    let mut suppress_depth = 0;
    let mut in_boilerplate_nav = false;
    let mut link_stack: Vec<String> = Vec::new();
    let mut badge_suppress_active = false;

    for event in parser {
        match event {
            Event::Start(Tag::Link { ref dest_url, .. }) => {
                if is_badge_url(dest_url) {
                    suppress_depth += 1;
                    badge_suppress_active = true;
                } else {
                    let clean_url = strip_tracking_params(dest_url);
                    link_stack.push(clean_url);
                    if suppress_depth == 0 {
                        result.push('[');
                    }
                }
            }
            Event::End(TagEnd::Link) => {
                if suppress_depth > 0 {
                    suppress_depth -= 1;
                    if suppress_depth == 0 {
                        badge_suppress_active = false;
                    }
                } else if in_boilerplate_nav {
                    in_boilerplate_nav = false;
                } else if let Some(clean_url) = link_stack.pop() {
                    result.push_str(&format!("]({})", clean_url));
                }
            }
            Event::Start(Tag::Image { ref dest_url, .. }) => {
                if is_badge_url(dest_url) || suppress_depth > 0 {
                    suppress_depth += 1;
                    badge_suppress_active = true;
                    if result.ends_with('[') {
                        result.pop();
                    }
                    link_stack.pop();
                } else {
                    result.push_str("![");
                }
            }
            Event::End(TagEnd::Image) => {
                if suppress_depth > 0 {
                    suppress_depth -= 1;
                    if suppress_depth == 0 {
                        badge_suppress_active = false;
                    }
                } else {
                    result.push(']');
                }
            }
            Event::Html(ref html) | Event::InlineHtml(ref html) => {
                let trimmed = html.trim();
                if trimmed.starts_with("<!--")
                    || trimmed.starts_with("<svg")
                    || trimmed.starts_with("<picture")
                    || trimmed.starts_with("<nav")
                    || (trimmed.starts_with("<img") && is_badge_url(trimmed))
                {
                    continue;
                } else if suppress_depth == 0 && !badge_suppress_active {
                    result.push_str(html);
                }
            }
            Event::Text(ref text) => {
                if is_badge_url(text) {
                    continue;
                }
                if suppress_depth == 0 && !badge_suppress_active {
                    let lower = text.trim().to_lowercase();
                    if lower == "back to top"
                        || lower == "back to top ↑"
                        || lower == "top"
                        || lower == "return to top"
                    {
                        in_boilerplate_nav = true;
                    } else {
                        result.push_str(text);
                    }
                }
            }
            Event::Code(ref code) => {
                if suppress_depth == 0 && !badge_suppress_active {
                    result.push('`');
                    result.push_str(code);
                    result.push('`');
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                if suppress_depth == 0 && !badge_suppress_active {
                    let prefix = "#".repeat(level as usize);
                    result.push_str(&format!("\n\n{} ", prefix));
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if suppress_depth == 0 && !badge_suppress_active {
                    result.push('\n');
                }
            }
            Event::Start(Tag::Paragraph) => {
                if suppress_depth == 0 && !badge_suppress_active {
                    result.push_str("\n\n");
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if suppress_depth == 0 && !badge_suppress_active {
                    result.push('\n');
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                if suppress_depth == 0 && !badge_suppress_active {
                    let lang = match kind {
                        CodeBlockKind::Fenced(l) => l.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    result.push_str(&format!("\n\n```{}\n", lang));
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if suppress_depth == 0 && !badge_suppress_active {
                    result.push_str("```\n");
                }
            }
            Event::Start(Tag::Item) => {
                if suppress_depth == 0 && !badge_suppress_active {
                    result.push_str("\n- ");
                }
            }
            Event::SoftBreak | Event::HardBreak
                if suppress_depth == 0 && !badge_suppress_active =>
            {
                result.push('\n');
            }
            _ => {}
        }
    }

    // Collapse 3+ consecutive newlines into max 2
    let mut normalized = String::with_capacity(result.len());
    let mut consecutive_newlines = 0;

    for ch in result.chars() {
        if ch == '\n' {
            consecutive_newlines += 1;
            if consecutive_newlines <= 2 {
                normalized.push(ch);
            }
        } else {
            consecutive_newlines = 0;
            normalized.push(ch);
        }
    }

    normalized.trim().to_string()
}

/// Cleans markdown and returns detailed token reduction metrics
pub fn clean_markdown_with_report(input: &str) -> CleanerReport {
    let raw_stats = calculate_stats(input);
    let cleaned_text = clean_markdown(input);
    let cleaned_stats = calculate_stats(&cleaned_text);

    let raw_tokens = raw_stats.estimated_tokens;
    let cleaned_tokens = cleaned_stats.estimated_tokens;
    let tokens_saved = raw_tokens.saturating_sub(cleaned_tokens);

    let reduction_percentage = if raw_tokens > 0 {
        (tokens_saved as f64 / raw_tokens as f64) * 100.0
    } else {
        0.0
    };

    CleanerReport {
        cleaned_text,
        raw_tokens,
        cleaned_tokens,
        tokens_saved,
        reduction_percentage,
    }
}

/// Generates a diff report comparing original markdown against cleaned token-thrifty markdown
pub fn generate_clean_diff(input: &str) -> String {
    let report = clean_markdown_with_report(input);
    let mut diff = String::new();

    diff.push_str(&format!(
        "=== PANKH TOKEN OPTIMIZATION DIFF REPORT ===\nRaw Tokens: {} | Clean Tokens: {} | Tokens Saved: {} ({:.1}% reduction)\n\n",
        report.raw_tokens, report.cleaned_tokens, report.tokens_saved, report.reduction_percentage
    ));

    diff.push_str("Estimated Dollar Savings (Input Tokens Saved):\n");
    let costs =
        crate::core::pricing::estimate_costs(report.raw_tokens, report.cleaned_tokens, None);
    for cost in costs {
        diff.push_str(&format!(
            "- {}: Saved ${:.4}\n",
            cost.model_name, cost.dollar_savings
        ));
    }
    diff.push('\n');

    diff.push_str("--- ORIGINAL RAW TEXT ---\n");
    diff.push_str(input.trim());
    diff.push_str("\n\n+++ CLEANED TOKEN-THRIFTY TEXT +++\n");
    diff.push_str(&report.cleaned_text);
    diff.push('\n');

    diff
}

/// Helper struct for flat heading scan
struct FlatHeading {
    level: u32,
    title: String,
    start_line: usize,
}

/// Helper function to build nested HeadingNode tree from flat list of headings
fn build_nested_heading_tree(
    flat: &[FlatHeading],
    lines: &[&str],
    total_raw_lines: usize,
) -> Vec<HeadingNode> {
    if flat.is_empty() {
        return Vec::new();
    }

    let mut nodes: Vec<HeadingNode> = Vec::new();
    let total_lines = lines.len();

    for (i, h) in flat.iter().enumerate() {
        let start_line = h.start_line;
        let end_line = flat[i + 1..]
            .iter()
            .find(|next_h| next_h.level <= h.level)
            .map(|next_h| next_h.start_line.saturating_sub(1))
            .unwrap_or(total_raw_lines);

        let section_slice = if start_line <= end_line && start_line <= total_lines {
            let start_idx = start_line.saturating_sub(1);
            let end_idx = end_line.min(total_lines);
            lines[start_idx..end_idx].join("\n")
        } else {
            String::new()
        };

        let stats = calculate_stats(&section_slice);

        nodes.push(HeadingNode {
            level: h.level,
            title: h.title.clone(),
            start_line,
            end_line,
            token_count: stats.estimated_tokens,
            character_count: stats.characters,
            children: Vec::new(),
        });
    }

    let mut root_nodes: Vec<HeadingNode> = Vec::new();

    for node in nodes {
        nest_node(&mut root_nodes, node, 0);
    }

    root_nodes
}

fn nest_node(siblings: &mut Vec<HeadingNode>, new_node: HeadingNode, depth: usize) {
    if depth >= MAX_NESTING_DEPTH {
        siblings.push(new_node);
        return;
    }
    if let Some(last) = siblings.last_mut() {
        if new_node.level > last.level {
            nest_node(&mut last.children, new_node, depth + 1);
            return;
        }
    }
    siblings.push(new_node);
}

fn count_sections(nodes: &[HeadingNode]) -> usize {
    nodes
        .iter()
        .fold(0, |acc, n| acc + 1 + count_sections(&n.children))
}

fn get_max_depth(nodes: &[HeadingNode]) -> u32 {
    nodes
        .iter()
        .map(|n| n.level.max(get_max_depth(&n.children)))
        .max()
        .unwrap_or(0)
}

/// Extracts structural outline of headings from Markdown text as a nested tree with exact line & token metrics
pub fn extract_outline(input: &str) -> OutlineTree {
    let frontmatter_offset = get_frontmatter_line_count(input);
    let content_without_frontmatter = strip_frontmatter(input);
    let lines: Vec<&str> = content_without_frontmatter.lines().collect();
    let total_raw_lines = input.lines().count();

    let mut flat_headings = Vec::new();
    let mut in_fenced_code = false;

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fenced_code = !in_fenced_code;
            continue;
        }

        if !in_fenced_code && trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|c| *c == '#').count() as u32;
            if (1..=6).contains(&level) {
                let title = trimmed[level as usize..].trim().to_string();
                if !title.is_empty() {
                    flat_headings.push(FlatHeading {
                        level,
                        title,
                        start_line: line_idx + 1 + frontmatter_offset,
                    });
                }
            }
        }
    }

    let raw_lines: Vec<&str> = input.lines().collect();
    let headings = build_nested_heading_tree(&flat_headings, &raw_lines, total_raw_lines);
    let total_sections = count_sections(&headings);
    let max_depth = get_max_depth(&headings);

    OutlineTree {
        headings,
        total_sections,
        max_depth,
    }
}

/// Extracts code snippets filtered optionally by language extension
pub fn extract_code_blocks(input: &str, lang_filter: Option<&str>) -> Vec<CodeBlock> {
    let content_without_frontmatter = strip_frontmatter(input);
    let mut code_blocks = Vec::new();
    let parser = parse_markdown(content_without_frontmatter);

    let mut in_code_block = false;
    let mut current_lang = String::new();
    let mut current_code = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                current_code.clear();
                current_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::from("text"),
                };
            }
            Event::Text(text) => {
                if in_code_block {
                    current_code.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) if in_code_block => {
                let lang_matches = match lang_filter {
                    Some(filter) => current_lang.eq_ignore_ascii_case(filter),
                    None => true,
                };

                if lang_matches {
                    code_blocks.push(CodeBlock {
                        language: if current_lang.is_empty() {
                            String::from("text")
                        } else {
                            current_lang.clone()
                        },
                        code: current_code.trim_end().to_string(),
                    });
                }
                in_code_block = false;
            }
            _ => {}
        }
    }

    code_blocks
}

/// Calculates metadata stats and token estimates for a document
pub fn calculate_stats(input: &str) -> DocumentStats {
    let lines = input.lines().count();
    let words = input.split_whitespace().count();
    let characters = input.chars().count();

    let estimated_tokens = characters.div_ceil(4);

    DocumentStats {
        lines,
        words,
        characters,
        estimated_tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontmatter_stripping() {
        let raw = "---\ntitle: Test\nauthor: Me\n---\n\n# Main Title\nBody text";
        let cleaned = clean_markdown(raw);
        assert!(!cleaned.contains("author: Me"));
        assert!(cleaned.contains("Main Title"));
    }

    #[test]
    fn test_frontmatter_line_offset_compensation() {
        let raw = "---\ntitle: Document\nauthor: Test\n---\n\n# Main Title\n\nContent here.";
        let outline = extract_outline(raw);
        assert_eq!(outline.headings.len(), 1);
        assert_eq!(outline.headings[0].start_line, 6);
    }

    #[test]
    fn test_badge_stripping() {
        let raw = "# Project\n[![Build Status](https://img.shields.io/badge.svg)](https://example.com)\n\nReal content here.";
        let cleaned = clean_markdown(raw);
        assert!(!cleaned.contains("img.shields.io"));
        assert!(cleaned.contains("Real content here."));
    }

    #[test]
    fn test_multiline_badge_link_stripping() {
        let raw = "# Header\n\n[![Build Status]\n(https://img.shields.io/badge.svg)]\n(https://example.com/build)\n\nContent";
        let cleaned = clean_markdown(raw);
        assert!(!cleaned.contains("shields.io"));
        assert!(cleaned.contains("Content"));
    }

    #[test]
    fn test_url_tracking_param_stripping_with_anchor() {
        let raw = "[Click Here](https://example.com/page?utm_source=twitter&ref=abc#section-1)";
        let cleaned = clean_markdown(raw);
        assert!(cleaned.contains("https://example.com/page#section-1"));
        assert!(!cleaned.contains("utm_source"));
    }

    #[test]
    fn test_nested_outline_tree_and_recursion_cap() {
        let raw = "# Title\n\nSome text\n\n## Section 1\n\n### SubSection\n";
        let outline = extract_outline(raw);
        assert_eq!(outline.total_sections, 3);
        assert_eq!(outline.max_depth, 3);
    }

    #[test]
    fn test_empty_input_cleaning() {
        let cleaned = clean_markdown("");
        assert!(cleaned.is_empty());
    }

    #[test]
    fn test_diff_clean_generation() {
        let raw = "# Header\n[![badge](https://img.shields.io/badge)](https://a.com)\n\nContent";
        let diff = generate_clean_diff(raw);
        assert!(diff.contains("PANKH TOKEN OPTIMIZATION DIFF REPORT"));
        assert!(diff.contains("Content"));
    }

    #[test]
    fn test_frontmatter_outline_section_tokens() {
        let raw =
            "---\ntitle: Doc\nauthor: Tester\n---\n\n# Main Title\n\nSection body text with words.";
        let outline = extract_outline(raw);
        assert_eq!(outline.headings.len(), 1);
        assert!(outline.headings[0].token_count > 0);
        assert_eq!(outline.headings[0].start_line, 6);
    }
}
