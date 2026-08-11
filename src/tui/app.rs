use arboard::Clipboard;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::core::agent::{
    calculate_stats, clean_markdown, extract_code_blocks, extract_outline, HeadingNode,
};
use crate::tui::ui::draw_ui;

/// Installs a global panic hook that safely restores standard terminal state (disables raw mode & exits alternate screen) if a panic occurs
pub fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));
}

pub fn flatten_headings(nodes: &[HeadingNode]) -> Vec<HeadingNode> {
    let mut flat = Vec::new();
    for node in nodes {
        flat.push(node.clone());
        flat.extend(flatten_headings(&node.children));
    }
    flat
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLink {
    pub line_number: usize,
    pub label: String,
    pub url: String,
}

pub fn extract_document_links(content: &str) -> Vec<DocumentLink> {
    let parser = pulldown_cmark::Parser::new(content);
    let mut links = Vec::new();
    let mut in_link = false;
    let mut current_label = String::new();
    let mut current_url = String::new();
    let mut line_counter = 1;

    for event in parser {
        match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { dest_url, .. }) => {
                in_link = true;
                current_label.clear();
                current_url = dest_url.to_string();
            }
            pulldown_cmark::Event::Text(text) => {
                line_counter += text.matches('\n').count();
                if in_link {
                    current_label.push_str(&text);
                }
            }
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Link) if in_link => {
                links.push(DocumentLink {
                    line_number: line_counter,
                    label: current_label.trim().to_string(),
                    url: current_url.clone(),
                });
                in_link = false;
            }
            _ => {}
        }
    }

    links
}

use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTheme {
    OceanDark,
    Dracula,
    Gruvbox,
    CleanLight,
}

impl AppTheme {
    pub fn name(&self) -> &'static str {
        match self {
            AppTheme::OceanDark => "Ocean Dark 🌙",
            AppTheme::Dracula => "Dracula 🧛",
            AppTheme::Gruvbox => "Gruvbox 🌲",
            AppTheme::CleanLight => "Clean Light ☀️",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            AppTheme::OceanDark => AppTheme::Dracula,
            AppTheme::Dracula => AppTheme::Gruvbox,
            AppTheme::Gruvbox => AppTheme::CleanLight,
            AppTheme::CleanLight => AppTheme::OceanDark,
        }
    }

    pub fn border_color(&self) -> Color {
        match self {
            AppTheme::OceanDark => Color::Blue,
            AppTheme::Dracula => Color::Magenta,
            AppTheme::Gruvbox => Color::Yellow,
            AppTheme::CleanLight => Color::Rgb(255, 180, 0),
        }
    }

    pub fn header_color(&self) -> Color {
        match self {
            AppTheme::OceanDark => Color::Cyan,
            AppTheme::Dracula => Color::LightRed,
            AppTheme::Gruvbox => Color::Rgb(251, 73, 52),
            AppTheme::CleanLight => Color::Rgb(255, 215, 0),
        }
    }

    pub fn text_color(&self) -> Color {
        match self {
            AppTheme::CleanLight => Color::Rgb(240, 245, 250),
            _ => Color::White,
        }
    }

    pub fn syntect_theme(&self) -> &'static str {
        match self {
            AppTheme::Gruvbox => "base16-eighties.dark",
            AppTheme::Dracula => "base16-mocha.dark",
            _ => "base16-ocean.dark",
        }
    }
}

pub struct App {
    pub raw_text: String,
    pub cleaned_text: String,
    pub scroll_offset: u16,
    pub rendered_line_count: usize,
    pub show_toc: bool,
    pub selected_toc_index: usize,
    pub headings: Vec<HeadingNode>,
    pub links: Vec<DocumentLink>,
    pub history_stack: Vec<(String, String, u16, Option<PathBuf>)>,
    pub active_path: Option<PathBuf>,
    pub search_active: bool,
    pub search_query: String,
    pub search_matches: Vec<u16>,
    pub current_search_match: usize,
    pub status_message: Option<String>,
    pub estimated_tokens: usize,
    pub should_quit: bool,
    pub theme: AppTheme,
    pub fuzzy_active: bool,
    pub fuzzy_query: String,
    pub fuzzy_files: Vec<(PathBuf, usize)>,
    pub fuzzy_matches: Vec<(PathBuf, usize)>,
    pub fuzzy_selected_index: usize,
}

impl App {
    pub fn new(content: &str) -> Self {
        let outline = extract_outline(content);
        let stats = calculate_stats(content);
        let cleaned = clean_markdown(content);
        let flat_headings = flatten_headings(&outline.headings);
        let links = extract_document_links(content);
        let theme = AppTheme::OceanDark;
        let rendered_lines = crate::tui::render::render_rich_markdown(content, "", theme);
        let rendered_line_count = rendered_lines.len();

        App {
            raw_text: content.to_string(),
            cleaned_text: cleaned,
            scroll_offset: 0,
            rendered_line_count,
            show_toc: false,
            selected_toc_index: 0,
            headings: flat_headings,
            links,
            history_stack: Vec::new(),
            active_path: None,
            search_active: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            current_search_match: 0,
            status_message: None,
            estimated_tokens: stats.estimated_tokens,
            should_quit: false,
            theme,
            fuzzy_active: false,
            fuzzy_query: String::new(),
            fuzzy_files: Vec::new(),
            fuzzy_matches: Vec::new(),
            fuzzy_selected_index: 0,
        }
    }

    pub fn reload_from_path(&mut self) {
        if let Some(ref path) = self.active_path {
            if let Ok(new_content) = crate::core::io::read_markdown_file_safe(path) {
                let outline = extract_outline(&new_content);
                let stats = calculate_stats(&new_content);
                let cleaned = clean_markdown(&new_content);
                let flat_headings = flatten_headings(&outline.headings);
                let links = extract_document_links(&new_content);
                self.raw_text = new_content;
                self.cleaned_text = cleaned;
                self.rendered_line_count =
                    crate::tui::render::render_rich_markdown(&self.raw_text, "", self.theme).len();
                self.headings = flat_headings;
                self.links = links;
                self.estimated_tokens = stats.estimated_tokens;
                self.status_message = Some(format!("[Watcher] Live reloaded {}", path.display()));
            }
        }
    }

    pub fn scroll_down(&mut self, amount: u16) {
        let max_scroll = self.rendered_line_count.saturating_sub(1) as u16;
        self.scroll_offset = self.scroll_offset.saturating_add(amount).min(max_scroll);
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn jump_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn jump_to_bottom(&mut self) {
        let max_scroll = self.rendered_line_count.saturating_sub(1) as u16;
        self.scroll_offset = max_scroll;
    }

    pub fn follow_current_line_link(&mut self) {
        let current_line = (self.scroll_offset as usize) + 1;
        let matching_link = self
            .links
            .iter()
            .find(|l| l.line_number == current_line || l.line_number.abs_diff(current_line) <= 2)
            .cloned();

        if let Some(link) = matching_link {
            if link.url.starts_with('#') {
                let target = link.url.trim_start_matches('#').to_lowercase();
                let heading_match = self.headings.iter().find(|h| {
                    h.title.to_lowercase().replace(' ', "-").contains(&target)
                        || h.title.to_lowercase().contains(&target)
                });

                if let Some(h) = heading_match {
                    self.scroll_offset = h.start_line.saturating_sub(1) as u16;
                    self.status_message = Some(format!("Jumped to section: {}", h.title));
                } else {
                    self.status_message = Some(format!("Section not found: {}", link.url));
                }
            } else {
                let (file_str, anchor_opt) = match link.url.split_once('#') {
                    Some((f, a)) => (f, Some(a)),
                    None => (link.url.as_str(), None),
                };

                let path = PathBuf::from(file_str);
                if path.exists() {
                    if let Ok(new_content) = crate::core::io::read_markdown_file_safe(&path) {
                        self.load_new_document(&new_content, file_str, Some(path));
                        if let Some(anchor) = anchor_opt {
                            let target = anchor.to_lowercase();
                            let heading_match = self.headings.iter().find(|h| {
                                h.title.to_lowercase().replace(' ', "-").contains(&target)
                                    || h.title.to_lowercase().contains(&target)
                            });
                            if let Some(h) = heading_match {
                                self.scroll_offset = h.start_line.saturating_sub(1) as u16;
                                self.status_message =
                                    Some(format!("Loaded file and jumped to: {}", h.title));
                            }
                        }
                    } else {
                        self.status_message = Some(format!("Failed to read file: {}", file_str));
                    }
                } else {
                    self.status_message = Some(format!("Link target not found: {}", link.url));
                }
            }
        }
    }

    pub fn load_new_document(&mut self, content: &str, title: &str, new_path: Option<PathBuf>) {
        self.history_stack.push((
            self.raw_text.clone(),
            title.to_string(),
            self.scroll_offset,
            self.active_path.clone(),
        ));
        let outline = extract_outline(content);
        let stats = calculate_stats(content);
        let cleaned = clean_markdown(content);
        let flat_headings = flatten_headings(&outline.headings);
        let links = extract_document_links(content);

        self.raw_text = content.to_string();
        self.cleaned_text = cleaned;
        self.rendered_line_count =
            crate::tui::render::render_rich_markdown(&self.raw_text, "", self.theme).len();
        self.scroll_offset = 0;
        self.headings = flat_headings;
        self.links = links;
        self.active_path = new_path;
        self.estimated_tokens = stats.estimated_tokens;
        self.status_message = Some(format!("Loaded document: {}", title));
    }

    pub fn backtrack_history(&mut self) {
        if let Some((prev_text, title, prev_scroll, prev_path)) = self.history_stack.pop() {
            let outline = extract_outline(&prev_text);
            let stats = calculate_stats(&prev_text);
            let cleaned = clean_markdown(&prev_text);
            let flat_headings = flatten_headings(&outline.headings);
            let links = extract_document_links(&prev_text);

            self.raw_text = prev_text;
            self.cleaned_text = cleaned;
            self.rendered_line_count =
                crate::tui::render::render_rich_markdown(&self.raw_text, "", self.theme).len();
            self.scroll_offset = prev_scroll;
            self.headings = flat_headings;
            self.links = links;
            self.active_path = prev_path;
            self.estimated_tokens = stats.estimated_tokens;
            self.status_message = Some(format!("Backtracked to: {}", title));
        } else {
            self.status_message = Some("Already at root document.".to_string());
        }
    }

    pub fn update_search_matches(&mut self) {
        self.search_matches.clear();
        if self.search_query.is_empty() {
            return;
        }

        let query = self.search_query.to_lowercase();
        for (idx, line) in self.raw_text.lines().enumerate() {
            if line.to_lowercase().contains(&query) {
                self.search_matches.push(idx as u16);
            }
        }

        if !self.search_matches.is_empty() {
            self.current_search_match = 0;
            self.scroll_offset = self.search_matches[0];
            self.status_message = Some(format!(
                "Match 1/{} (Press n/N to navigate)",
                self.search_matches.len()
            ));
        } else {
            self.status_message = Some("No matches found".to_string());
        }
    }

    pub fn next_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.current_search_match = (self.current_search_match + 1) % self.search_matches.len();
        self.scroll_offset = self.search_matches[self.current_search_match];
        self.status_message = Some(format!(
            "Match {}/{}",
            self.current_search_match + 1,
            self.search_matches.len()
        ));
    }

    pub fn prev_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.current_search_match == 0 {
            self.current_search_match = self.search_matches.len() - 1;
        } else {
            self.current_search_match -= 1;
        }
        self.scroll_offset = self.search_matches[self.current_search_match];
        self.status_message = Some(format!(
            "Match {}/{}",
            self.current_search_match + 1,
            self.search_matches.len()
        ));
    }

    pub fn toggle_toc(&mut self) {
        self.show_toc = !self.show_toc;
    }

    pub fn copy_clean_text_to_clipboard(&mut self) {
        match Clipboard::new() {
            Ok(mut clipboard) => {
                if clipboard.set_text(&self.cleaned_text).is_ok() {
                    self.status_message =
                        Some("Copied token-thrifty text to clipboard!".to_string());
                } else {
                    self.status_message = Some("Failed to write clipboard text.".to_string());
                }
            }
            Err(_) => {
                self.status_message = Some("Clipboard unavailable.".to_string());
            }
        }
    }

    pub fn cycle_theme(&mut self) {
        self.theme = self.theme.next();
        self.status_message = Some(format!("Theme set to: {}", self.theme.name()));
    }

    pub fn open_fuzzy_finder(&mut self) {
        self.fuzzy_active = true;
        self.fuzzy_query.clear();
        self.fuzzy_selected_index = 0;
        self.fuzzy_files.clear();

        let mut paths = Vec::new();
        fn collect_md(dir: &Path, acc: &mut Vec<PathBuf>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        if let Some(ext) = p.extension() {
                            if ext.eq_ignore_ascii_case("md")
                                || ext.eq_ignore_ascii_case("markdown")
                            {
                                acc.push(p);
                            }
                        }
                    } else if p.is_dir() {
                        let name = p.file_name().unwrap_or_default().to_string_lossy();
                        if !name.starts_with('.') && name != "target" && name != "node_modules" {
                            collect_md(&p, acc);
                        }
                    }
                }
            }
        }

        collect_md(Path::new("."), &mut paths);
        for p in paths {
            if let Ok(c) = crate::core::io::read_markdown_file_safe(&p) {
                let stats = calculate_stats(&c);
                self.fuzzy_files.push((p, stats.estimated_tokens));
            }
        }
        self.update_fuzzy_matches();
    }

    pub fn update_fuzzy_matches(&mut self) {
        self.fuzzy_matches.clear();
        let query = self.fuzzy_query.to_lowercase();
        for (path, tokens) in &self.fuzzy_files {
            let path_str = path.display().to_string().to_lowercase();
            if query.is_empty() || path_str.contains(&query) {
                self.fuzzy_matches.push((path.clone(), *tokens));
            }
        }
        if self.fuzzy_selected_index >= self.fuzzy_matches.len() {
            self.fuzzy_selected_index = 0;
        }
    }

    pub fn select_fuzzy_match(&mut self) {
        if self.fuzzy_matches.is_empty() {
            return;
        }
        let (selected_path, _) = self.fuzzy_matches[self.fuzzy_selected_index].clone();
        if let Ok(content) = crate::core::io::read_markdown_file_safe(&selected_path) {
            let title = selected_path.display().to_string();
            self.load_new_document(&content, &title, Some(selected_path));
            self.fuzzy_active = false;
        }
    }

    pub fn copy_focused_code_block(&mut self) {
        let code_blocks = extract_code_blocks(&self.raw_text, None);
        if code_blocks.is_empty() {
            self.status_message = Some("No code blocks found in document.".to_string());
            return;
        }

        // Copy the first code block or the block nearest to current scroll offset
        let code_to_copy = &code_blocks[0].code;
        match Clipboard::new() {
            Ok(mut clipboard) => {
                if clipboard.set_text(code_to_copy).is_ok() {
                    self.status_message = Some(format!(
                        "Copied code block ({}) to clipboard!",
                        code_blocks[0].language
                    ));
                } else {
                    self.status_message = Some("Failed to copy code block.".to_string());
                }
            }
            Err(_) => {
                self.status_message = Some("Clipboard unavailable.".to_string());
            }
        }
    }

    pub fn clear_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.search_matches.clear();
        self.current_search_match = 0;
        self.status_message = Some("Search cleared".to_string());
    }

    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        if self.fuzzy_active {
            match code {
                KeyCode::Esc => {
                    self.fuzzy_active = false;
                }
                KeyCode::Enter => {
                    self.select_fuzzy_match();
                }
                KeyCode::Up | KeyCode::Char('k') if modifiers.contains(KeyModifiers::CONTROL) => {
                    if self.fuzzy_selected_index > 0 {
                        self.fuzzy_selected_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') if modifiers.contains(KeyModifiers::CONTROL) => {
                    if !self.fuzzy_matches.is_empty() {
                        self.fuzzy_selected_index =
                            (self.fuzzy_selected_index + 1).min(self.fuzzy_matches.len() - 1);
                    }
                }
                KeyCode::Backspace => {
                    self.fuzzy_query.pop();
                    self.update_fuzzy_matches();
                }
                KeyCode::Char(c) => {
                    self.fuzzy_query.push(c);
                    self.update_fuzzy_matches();
                }
                _ => {}
            }
            return;
        }

        if self.search_active {
            match code {
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                }
                KeyCode::Enter => {
                    self.search_active = false;
                    self.update_search_matches();
                }
                KeyCode::Esc => {
                    self.clear_search();
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                }
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('p') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.open_fuzzy_finder();
            }
            KeyCode::Char('f') => {
                self.open_fuzzy_finder();
            }
            KeyCode::Char('t') => {
                self.cycle_theme();
            }
            KeyCode::Char('y') => {
                self.copy_focused_code_block();
            }
            KeyCode::Esc => {
                if !self.search_query.is_empty()
                    || !self.search_matches.is_empty()
                    || self.search_active
                {
                    self.clear_search();
                } else if self.status_message.is_some() {
                    self.status_message = None;
                } else if self.show_toc {
                    self.show_toc = false;
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('c') if !self.search_query.is_empty() => {
                self.clear_search();
            }
            KeyCode::Enter => {
                self.follow_current_line_link();
            }
            KeyCode::Backspace => {
                self.backtrack_history();
            }
            KeyCode::Char('g') => {
                self.jump_to_top();
            }
            KeyCode::Char('G') => {
                self.jump_to_bottom();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.show_toc && !self.headings.is_empty() {
                    self.selected_toc_index = (self.selected_toc_index + 1) % self.headings.len();
                    self.scroll_offset = (self.headings[self.selected_toc_index]
                        .start_line
                        .saturating_sub(1)) as u16;
                } else {
                    self.scroll_down(1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.show_toc && !self.headings.is_empty() {
                    if self.selected_toc_index == 0 {
                        self.selected_toc_index = self.headings.len() - 1;
                    } else {
                        self.selected_toc_index -= 1;
                    }
                    self.scroll_offset = (self.headings[self.selected_toc_index]
                        .start_line
                        .saturating_sub(1)) as u16;
                } else {
                    self.scroll_up(1);
                }
            }
            KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_down(10);
            }
            KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_up(10);
            }
            KeyCode::Char('n') => {
                self.next_search_match();
            }
            KeyCode::Char('N') => {
                self.prev_search_match();
            }
            KeyCode::Tab | KeyCode::Char('b') => {
                self.toggle_toc();
            }
            KeyCode::Char('/') => {
                self.search_active = true;
                self.search_query.clear();
            }
            KeyCode::Char('a') => {
                self.copy_clean_text_to_clipboard();
            }
            _ => {}
        }
    }
}

/// Runs interactive TUI terminal application with optional live watcher
pub fn run_tui(content: &str, paths: &[PathBuf], watch: bool) -> anyhow::Result<()> {
    install_panic_hook();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(content);
    if let Some(first_path) = paths.first() {
        app.active_path = Some(first_path.clone());
    }

    let needs_reload = Arc::new(AtomicBool::new(false));

    if watch && !paths.is_empty() {
        let reload_flag = Arc::clone(&needs_reload);
        let _ = crate::core::watcher::watch_paths(paths, move || {
            reload_flag.store(true, Ordering::SeqCst);
        });
        app.status_message = Some("[Watcher Active] Live-reloading on save...".to_string());
    }

    loop {
        if needs_reload.swap(false, Ordering::SeqCst) {
            app.reload_from_path();
        }

        terminal.draw(|f| draw_ui(f, &app))?;

        if app.should_quit {
            break;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    app.handle_key(key.code, key.modifiers);
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollDown => app.scroll_down(3),
                    MouseEventKind::ScrollUp => app.scroll_up(3),
                    MouseEventKind::Down(MouseButton::Left) => {
                        let size = terminal.size()?;
                        if app.show_toc && mouse.column < size.width / 4 {
                            let idx = mouse.row.saturating_sub(1) as usize;
                            if idx < app.headings.len() {
                                app.selected_toc_index = idx;
                                app.scroll_offset =
                                    app.headings[idx].start_line.saturating_sub(1) as u16;
                            }
                        } else {
                            app.follow_current_line_link();
                        }
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_app_state() {
        let mut app = App::new(
            "# Test Header\n\nContent line 1\nContent line 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8",
        );
        assert_eq!(app.scroll_offset, 0);
        app.scroll_down(5);
        assert_eq!(app.scroll_offset, 5);
        app.scroll_up(2);
        assert_eq!(app.scroll_offset, 3);

        // Test boundary clamping
        app.scroll_down(100);
        let max_expected = app.rendered_line_count.saturating_sub(1) as u16;
        assert_eq!(app.scroll_offset, max_expected);

        assert!(!app.show_toc);
        app.toggle_toc();
        assert!(app.show_toc);
    }

    #[test]
    fn test_vim_motions_and_search() {
        let mut app = App::new("# Title\nLine 1\nLine 2\nTarget Match\nLine 4");
        app.jump_to_bottom();
        assert_eq!(app.scroll_offset, 4);
        app.jump_to_top();
        assert_eq!(app.scroll_offset, 0);

        app.search_query = "Target".to_string();
        app.update_search_matches();
        assert_eq!(app.search_matches.len(), 1);
    }

    #[test]
    fn test_tui_anchor_link_jump() {
        let content = "# Overview\n\nSee [Installation](#installation) below.\n\n## Installation\n\nDetails here.";
        let mut app = App::new(content);
        assert_eq!(app.links.len(), 1);
        assert_eq!(app.links[0].url, "#installation");

        app.scroll_offset = 2;
        app.follow_current_line_link();
        assert_eq!(app.scroll_offset, 4);
    }

    #[test]
    fn test_tui_file_history_backtrack() {
        let mut app = App::new("# Doc 1\n\nContent 1");
        app.load_new_document("# Doc 2\n\nContent 2", "doc2.md", None);
        assert_eq!(app.history_stack.len(), 1);
        assert!(app.raw_text.contains("Doc 2"));

        app.backtrack_history();
        assert_eq!(app.history_stack.len(), 0);
        assert!(app.raw_text.contains("Doc 1"));
    }
}
