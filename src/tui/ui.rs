use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::App;
use crate::tui::render::render_rich_markdown;

pub fn draw_ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1), // Footer status bar
        ])
        .split(f.area());

    let main_area = chunks[0];
    let status_area = chunks[1];

    if app.show_toc {
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // TOC Sidebar
                Constraint::Percentage(75), // Main Content Viewport
            ])
            .split(main_area);

        draw_toc_sidebar(f, h_chunks[0], app);
        draw_main_viewport(f, h_chunks[1], app);
    } else {
        draw_main_viewport(f, main_area, app);
    }

    draw_status_bar(f, status_area, app);
}

fn draw_toc_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .headings
        .iter()
        .enumerate()
        .map(|(idx, h)| {
            let indent = " ".repeat((h.level.saturating_sub(1) * 2) as usize);
            let style = if idx == app.selected_toc_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            ListItem::new(Line::from(Span::styled(
                format!("{}• {}", indent, h.title),
                style,
            )))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" TOC (Outline) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(list, area);
}

fn draw_main_viewport(f: &mut Frame, area: Rect, app: &App) {
    let lines = render_rich_markdown(&app.raw_text, &app.search_query);

    let title = if app.search_active {
        format!(" Search: {}_ ", app.search_query)
    } else {
        String::from(" Pankh Reader (Human Mode) ")
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .scroll((app.scroll_offset, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let status_text = if app.search_active {
        format!(
            " [Search Mode] Query: \"{}\"_ | [Enter]: Find | [Esc]: Cancel Search | [Ctrl+C]: Quit",
            app.search_query
        )
    } else {
        match &app.status_message {
            Some(msg) => msg.clone(),
            None => {
                if !app.search_query.is_empty() {
                    format!(
                        " Search: \"{}\" ({}/{} matches) | [n/N]: Match | [Esc/c]: Clear Search | [j/k]: Scroll | [q]: Quit",
                        app.search_query,
                        if app.search_matches.is_empty() { 0 } else { app.current_search_match + 1 },
                        app.search_matches.len()
                    )
                } else {
                    format!(
                        " Line: {}/{} | Est. Tokens: {} | [j/k/g/G]: Scroll | [Tab/b]: TOC | [a]: Copy Clean | [/]: Search | [q]: Quit",
                        app.scroll_offset + 1,
                        app.rendered_line_count,
                        app.estimated_tokens
                    )
                }
            }
        }
    };

    let p = Paragraph::new(Line::from(Span::styled(
        status_text,
        Style::default().bg(Color::DarkGray).fg(Color::White),
    )));

    f.render_widget(p, area);
}
