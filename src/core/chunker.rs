use crate::core::agent::{calculate_stats, extract_outline};
use crate::tui::app::flatten_headings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarkdownChunk {
    pub chunk_index: usize,
    pub title: String,
    pub start_line: usize,
    pub end_line: usize,
    pub token_count: usize,
    pub content: String,
}

/// Slices a Markdown document at section/heading boundaries under max_tokens limit per chunk
pub fn chunk_markdown(input: &str, max_tokens: usize) -> Vec<MarkdownChunk> {
    let outline = extract_outline(input);
    let flat_headings = flatten_headings(&outline.headings);
    let lines: Vec<&str> = input.lines().collect();

    if lines.is_empty() {
        return Vec::new();
    }

    if flat_headings.is_empty() {
        return chunk_lines_fallback(&lines, "Document", 1, max_tokens);
    }

    // Build explicit section list including preamble (if text exists before first heading)
    let mut sections: Vec<(String, usize, usize)> = Vec::new();

    let first_heading_start = flat_headings[0].start_line.saturating_sub(1);
    if first_heading_start > 0 {
        sections.push(("Preamble".to_string(), 0, first_heading_start));
    }

    for (idx, heading) in flat_headings.iter().enumerate() {
        let start = heading.start_line.saturating_sub(1);
        let end = if idx + 1 < flat_headings.len() {
            flat_headings[idx + 1].start_line.saturating_sub(1)
        } else {
            lines.len()
        };
        if start < lines.len() {
            sections.push((heading.title.clone(), start, end.min(lines.len())));
        }
    }

    let mut chunks = Vec::new();
    let mut current_chunk_lines: Vec<&str> = Vec::new();
    let mut current_chunk_tokens = 0;
    let mut chunk_start_line = 1;
    let mut chunk_title = String::new();

    for (title, start, end) in sections {
        if start >= lines.len() {
            continue;
        }

        let section_lines = &lines[start..end];
        let section_text = section_lines.join("\n");
        let section_stats = calculate_stats(&section_text);

        // If a single section exceeds max_tokens on its own, flush current and line-chunk section
        if section_stats.estimated_tokens > max_tokens {
            if !current_chunk_lines.is_empty() {
                let chunk_content = current_chunk_lines.join("\n");
                let chunk_end_line = chunk_start_line + current_chunk_lines.len().saturating_sub(1);
                chunks.push(MarkdownChunk {
                    chunk_index: chunks.len() + 1,
                    title: chunk_title.clone(),
                    start_line: chunk_start_line,
                    end_line: chunk_end_line,
                    token_count: current_chunk_tokens,
                    content: chunk_content,
                });
                current_chunk_lines.clear();
                current_chunk_tokens = 0;
            }

            let sub_chunks = chunk_lines_fallback(section_lines, &title, start + 1, max_tokens);
            for mut sc in sub_chunks {
                sc.chunk_index = chunks.len() + 1;
                chunks.push(sc);
            }
            continue;
        }

        if current_chunk_tokens + section_stats.estimated_tokens > max_tokens
            && !current_chunk_lines.is_empty()
        {
            let chunk_content = current_chunk_lines.join("\n");
            let chunk_end_line = chunk_start_line + current_chunk_lines.len().saturating_sub(1);
            chunks.push(MarkdownChunk {
                chunk_index: chunks.len() + 1,
                title: chunk_title.clone(),
                start_line: chunk_start_line,
                end_line: chunk_end_line,
                token_count: current_chunk_tokens,
                content: chunk_content,
            });

            current_chunk_lines.clear();
            current_chunk_tokens = 0;
        }

        if current_chunk_lines.is_empty() {
            chunk_start_line = start + 1;
            chunk_title = title;
        }

        current_chunk_lines.extend_from_slice(section_lines);
        current_chunk_tokens += section_stats.estimated_tokens;
    }

    if !current_chunk_lines.is_empty() {
        let chunk_content = current_chunk_lines.join("\n");
        let chunk_end_line = chunk_start_line + current_chunk_lines.len().saturating_sub(1);
        chunks.push(MarkdownChunk {
            chunk_index: chunks.len() + 1,
            title: chunk_title,
            start_line: chunk_start_line,
            end_line: chunk_end_line,
            token_count: current_chunk_tokens,
            content: chunk_content,
        });
    }

    chunks
}

fn chunk_lines_fallback(
    lines: &[&str],
    title: &str,
    start_line_offset: usize,
    max_tokens: usize,
) -> Vec<MarkdownChunk> {
    let mut chunks = Vec::new();
    let mut acc_lines: Vec<&str> = Vec::new();
    let mut acc_tokens = 0;
    let mut chunk_start = start_line_offset;

    for (i, line) in lines.iter().enumerate() {
        let line_stats = calculate_stats(line);
        if acc_tokens + line_stats.estimated_tokens > max_tokens && !acc_lines.is_empty() {
            let content = acc_lines.join("\n");
            let end_line = chunk_start + acc_lines.len().saturating_sub(1);
            chunks.push(MarkdownChunk {
                chunk_index: chunks.len() + 1,
                title: title.to_string(),
                start_line: chunk_start,
                end_line,
                token_count: acc_tokens,
                content,
            });
            acc_lines.clear();
            acc_tokens = 0;
            chunk_start = start_line_offset + i;
        }
        acc_lines.push(line);
        acc_tokens += line_stats.estimated_tokens;
    }

    if !acc_lines.is_empty() {
        let content = acc_lines.join("\n");
        let end_line = chunk_start + acc_lines.len().saturating_sub(1);
        chunks.push(MarkdownChunk {
            chunk_index: chunks.len() + 1,
            title: title.to_string(),
            start_line: chunk_start,
            end_line,
            token_count: acc_tokens,
            content,
        });
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunking_under_token_limit() {
        let raw = "# Section 1\nLine 1\nLine 2\n\n## Section 2\nLine 3\nLine 4";
        let chunks = chunk_markdown(raw, 100);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].chunk_index, 1);
    }

    #[test]
    fn test_single_large_section_chunking() {
        let raw = "# Title\nVery long section content text repeating multiple times to force token overflow.";
        let chunks = chunk_markdown(raw, 5);
        assert!(!chunks.is_empty());
        assert!(chunks.len() > 1);
    }

    #[test]
    fn test_chunk_preamble_preservation() {
        let raw = "Introductory preamble line 1\nIntroductory preamble line 2\n\n# Section 1\nSection body";
        let chunks = chunk_markdown(raw, 50);
        assert!(chunks
            .iter()
            .any(|c| c.title == "Preamble" || c.content.contains("preamble")));
    }
}
