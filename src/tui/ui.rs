use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
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

    if app.fuzzy_active {
        draw_fuzzy_finder_modal(f, f.area(), app);
    }
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
                Style::default().fg(app.theme.header_color())
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
            .border_style(Style::default().fg(app.theme.border_color())),
    );

    f.render_widget(list, area);
}

fn draw_main_viewport(f: &mut Frame, area: Rect, app: &App) {
    let lines = render_rich_markdown(&app.raw_text, &app.search_query, app.theme);

    let title = if app.search_active {
        format!(" Search: {}_ ", app.search_query)
    } else if let Some(ref path) = app.active_path {
        format!(" Pankh Reader - {} ({}) ", path.display(), app.theme.name())
    } else {
        format!(" Pankh Reader ({}) ", app.theme.name())
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.border_color())),
        )
        .scroll((app.scroll_offset, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn draw_fuzzy_finder_modal(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(65, 65, area);
    let items: Vec<ListItem> = app
        .fuzzy_matches
        .iter()
        .enumerate()
        .map(|(idx, (path, tokens))| {
            let style = if idx == app.fuzzy_selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.text_color())
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("📄 {} ", path.display()), style),
                Span::styled(
                    format!("({} tokens)", tokens),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let title = format!(
        " Workspace Fuzzy Finder: {}_ (Ctrl+J/K: Nav, Enter: Open, Esc: Close) ",
        app.fuzzy_query
    );
    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.border_color())),
    );

    f.render_widget(Clear, popup_area);
    f.render_widget(list, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let status_text = if app.fuzzy_active {
        " [Fuzzy Finder] Type to filter | Ctrl+J/K: Navigate | Enter: Open | Esc: Close".to_string()
    } else if app.search_active {
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
                        " Line: {}/{} | Est. Tokens: {} | [Ctrl+P/f]: Finder | [t]: Theme | [y]: Copy Code | [Tab/b]: TOC | [/]: Search | [q]: Quit",
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
