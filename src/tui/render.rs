use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use unicode_width::UnicodeWidthStr;

use crate::core::parser::parse_markdown;
use once_cell::sync::Lazy;
use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};

use crate::tui::app::AppTheme;

pub static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
pub static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Renders raw Markdown into rich styled Ratatui Lines with header hierarchy, blockquotes, nested lists, task checkboxes, tables, and syntax highlighting
pub fn render_rich_markdown(
    raw_text: &str,
    search_query: &str,
    theme: AppTheme,
) -> Vec<Line<'static>> {
    let ps = &*SYNTAX_SET;
    let ts = &*THEME_SET;
    let syntect_theme = &ts.themes[theme.syntect_theme()];

    let mut lines: Vec<Line> = Vec::new();
    let mut in_code_block = false;
    let mut current_code_lang = String::new();
    let mut code_lines: Vec<String> = Vec::new();
    let mut in_blockquote = false;
    let mut current_heading_level = 0;
    let mut list_depth: usize = 0;

    // Table state
    let mut in_table = false;
    let mut in_table_head = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();

    let parser = parse_markdown(raw_text);

    for event in parser {
        match event {
            Event::Start(Tag::Table(_)) => {
                in_table = true;
                table_rows.clear();
            }
            Event::End(TagEnd::Table) => {
                if in_table && !table_rows.is_empty() {
                    // Calculate exact visual column display widths using unicode-width
                    let col_count = table_rows.iter().map(|r| r.len()).max().unwrap_or(0);
                    let mut col_widths = vec![0; col_count];

                    for row in &table_rows {
                        for (c_idx, cell) in row.iter().enumerate() {
                            if c_idx < col_widths.len() {
                                col_widths[c_idx] = col_widths[c_idx].max(cell.width());
                            }
                        }
                    }

                    // Render top border
                    let top_border = format!(
                        "┌{}┐",
                        col_widths
                            .iter()
                            .map(|w| "─".repeat(w + 2))
                            .collect::<Vec<_>>()
                            .join("┬")
                    );
                    lines.push(Line::from(Span::styled(
                        top_border,
                        Style::default().fg(Color::DarkGray),
                    )));

                    // Render rows
                    for (r_idx, row) in table_rows.iter().enumerate() {
                        let mut line_spans = Vec::new();
                        line_spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));

                        for (c_idx, width) in col_widths.iter().enumerate().take(col_count) {
                            let cell_text = row.get(c_idx).cloned().unwrap_or_default();
                            let cell_width = cell_text.width();
                            let padding = " ".repeat(width.saturating_sub(cell_width));
                            let padded = format!(" {}{} ", cell_text, padding);

                            let style = if r_idx == 0 {
                                Style::default()
                                    .fg(theme.header_color())
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(theme.text_color())
                            };

                            line_spans.push(Span::styled(padded, style));
                            line_spans
                                .push(Span::styled("│", Style::default().fg(Color::DarkGray)));
                        }
                        lines.push(Line::from(line_spans));

                        if r_idx == 0 {
                            let sep = format!(
                                "├{}┤",
                                col_widths
                                    .iter()
                                    .map(|w| "─".repeat(w + 2))
                                    .collect::<Vec<_>>()
                                    .join("┼")
                            );
                            lines.push(Line::from(Span::styled(
                                sep,
                                Style::default().fg(Color::DarkGray),
                            )));
                        }
                    }

                    // Render bottom border
                    let bottom_border = format!(
                        "└{}┘",
                        col_widths
                            .iter()
                            .map(|w| "─".repeat(w + 2))
                            .collect::<Vec<_>>()
                            .join("┴")
                    );
                    lines.push(Line::from(Span::styled(
                        bottom_border,
                        Style::default().fg(Color::DarkGray),
                    )));

                    in_table = false;
                }
            }
            Event::Start(Tag::TableHead) => {
                in_table_head = true;
                current_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                in_table_head = false;
                if !current_row.is_empty() {
                    table_rows.push(current_row.clone());
                    current_row.clear();
                }
            }
            Event::Start(Tag::TableRow) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                if !current_row.is_empty() && !in_table_head {
                    table_rows.push(current_row.clone());
                    current_row.clear();
                }
            }
            Event::Start(Tag::TableCell) => {
                current_cell.clear();
            }
            Event::End(TagEnd::TableCell) => {
                current_row.push(current_cell.trim().to_string());
                current_cell.clear();
            }
            Event::Start(Tag::Heading { level, .. }) => {
                current_heading_level = level as u32;
            }
            Event::End(TagEnd::Heading(_)) => {
                current_heading_level = 0;
            }
            Event::Start(Tag::BlockQuote(_)) => {
                in_blockquote = true;
            }
            Event::End(TagEnd::BlockQuote) => {
                in_blockquote = false;
            }
            Event::Start(Tag::List(_)) => {
                list_depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
            }
            Event::TaskListMarker(checked) => {
                let checkbox = if checked { "[✓] " } else { "[ ] " };
                let color = if checked {
                    Color::Green
                } else {
                    Color::DarkGray
                };
                lines.push(Line::from(Span::styled(
                    checkbox,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_lines.clear();
                current_code_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::from("text"),
                };
            }
            Event::End(TagEnd::CodeBlock) => {
                if in_code_block {
                    let syntax = ps
                        .find_syntax_by_extension(&current_code_lang)
                        .unwrap_or_else(|| ps.find_syntax_plain_text());
                    let mut highlighter = HighlightLines::new(syntax, syntect_theme);

                    let lang_tag = if current_code_lang.trim().is_empty() {
                        "CODE".to_string()
                    } else {
                        current_code_lang.trim().to_uppercase()
                    };

                    lines.push(Line::from(vec![
                        Span::styled("┌─ [ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            lang_tag,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            " ] ────────────────────────────────────────┐",
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));

                    for code_line in &code_lines {
                        let ranges = highlighter
                            .highlight_line(code_line, &ps)
                            .unwrap_or_default();
                        let spans: Vec<Span> = ranges
                            .into_iter()
                            .map(|(style, text)| {
                                let fg = Color::Rgb(
                                    style.foreground.r,
                                    style.foreground.g,
                                    style.foreground.b,
                                );
                                Span::styled(text.to_string(), Style::default().fg(fg))
                            })
                            .collect();
                        lines.push(Line::from(spans));
                    }

                    lines.push(Line::from(Span::styled(
                        "└──────────────────────────────────────────────────┘",
                        Style::default().fg(Color::DarkGray),
                    )));
                    in_code_block = false;
                }
            }
            Event::Text(ref text) => {
                if in_table {
                    current_cell.push_str(text);
                    continue;
                }

                if in_code_block {
                    for line in text.lines() {
                        code_lines.push(line.to_string());
                    }
                    continue;
                }

                for raw_line in text.lines() {
                    let mut line_spans = Vec::new();

                    if in_blockquote {
                        line_spans.push(Span::styled(
                            "│ ",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ));
                    }

                    if current_heading_level > 0 {
                        let style = match current_heading_level {
                            1 => Style::default()
                                .fg(Color::Magenta)
                                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                            2 => Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                            3 => Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                            4 => Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                            5 => Style::default()
                                .fg(Color::Blue)
                                .add_modifier(Modifier::BOLD),
                            _ => Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                        };
                        let prefix = "#".repeat(current_heading_level as usize);
                        line_spans.push(Span::styled(format!("{} ", prefix), style));
                        line_spans.push(Span::styled(raw_line.to_string(), style));
                    } else if !search_query.is_empty()
                        && raw_line
                            .to_lowercase()
                            .contains(&search_query.to_lowercase())
                    {
                        line_spans.push(Span::styled(
                            raw_line.to_string(),
                            Style::default()
                                .bg(Color::Yellow)
                                .fg(Color::Black)
                                .add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        line_spans.push(Span::styled(
                            raw_line.to_string(),
                            Style::default().fg(theme.text_color()),
                        ));
                    }

                    lines.push(Line::from(line_spans));
                }
            }
            Event::Start(Tag::Item) => {
                let bullet = match list_depth {
                    0 | 1 => "  • ",
                    2 => "    ⁃ ",
                    _ => "      ▪ ",
                };
                lines.push(Line::from(Span::styled(
                    bullet,
                    Style::default().fg(Color::Cyan),
                )));
            }
            Event::Code(ref code) => {
                if in_table {
                    current_cell.push_str(&format!("`{}`", code));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!(" `{}` ", code),
                        Style::default()
                            .bg(Color::Rgb(40, 44, 52))
                            .fg(Color::LightGreen),
                    )));
                }
            }
            Event::Rule => {
                lines.push(Line::from(Span::styled(
                    "──────────────",
                    Style::default().fg(Color::DarkGray),
                )));
            }
            _ => {}
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::raw(raw_text.to_string())));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rich_render_table() {
        let raw = "| Name | Role |\n|---|---|\n| Pankh | Reader |";
        let lines = render_rich_markdown(raw, "", AppTheme::OceanDark);
        assert!(lines.iter().any(|l| l.to_string().contains("┌")));
        assert!(lines.iter().any(|l| l.to_string().contains("Pankh")));
    }

    #[test]
    fn test_rich_render_unicode_table_alignment() {
        let raw = "| Icon | Name |\n|---|---|\n| 🪶 | Pankh Reader |";
        let lines = render_rich_markdown(raw, "", AppTheme::OceanDark);
        assert!(lines.iter().any(|l| l.to_string().contains("┌")));
        assert!(lines.iter().any(|l| l.to_string().contains("🪶")));
    }

    #[test]
    fn test_rich_render_task_list() {
        let raw = "- [x] Done\n- [ ] Todo";
        let lines = render_rich_markdown(raw, "", AppTheme::OceanDark);
        assert!(lines.iter().any(|l| l.to_string().contains("[✓]")));
    }
}
