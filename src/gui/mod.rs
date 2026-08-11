use std::path::Path;
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

use crate::core::agent::{calculate_stats, extract_outline};
use crate::core::io::read_markdown_file_safe;

/// Converts Markdown text into clean HTML string
pub fn markdown_to_html(md: &str) -> String {
    let parser = pulldown_cmark::Parser::new(md);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output
}

/// Generates a standalone, rich, responsive HTML5 desktop reader application string
pub fn generate_gui_html(content: &str, file_path: Option<&Path>) -> String {
    let rendered_body = markdown_to_html(content);
    let outline = extract_outline(content);
    let stats = calculate_stats(content);

    let doc_title = file_path
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Untitled Document".to_string());

    let path_str = file_path
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "No file loaded".to_string());

    // Generate TOC links
    let mut toc_html = String::new();
    for node in &outline.headings {
        let slug = node
            .title
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
            .replace(' ', "-");
        let indent = (node.level.saturating_sub(1)) * 12;
        toc_html.push_str(&format!(
            r##"<a href="#heading-{}" class="toc-link level-{}" style="padding-left: {}px;">{}</a>"##,
            slug, node.level, indent + 8, html_escape(&node.title)
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en" data-theme="ocean">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Pankh Reader - {}</title>
  <style>
    :root {{
      --bg: #0f172a;
      --card-bg: #1e293b;
      --text: #f8fafc;
      --muted: #94a3b8;
      --accent: #38bdf8;
      --border: #334155;
      --code-bg: #020617;
    }}
    [data-theme="dracula"] {{
      --bg: #282a36;
      --card-bg: #44475a;
      --text: #f8f8f2;
      --muted: #6272a4;
      --accent: #ff79c6;
      --border: #6272a4;
      --code-bg: #191a21;
    }}
    [data-theme="gruvbox"] {{
      --bg: #282828;
      --card-bg: #3c3836;
      --text: #ebdbb2;
      --muted: #a89984;
      --accent: #fabd2f;
      --border: #504945;
      --code-bg: #1d2021;
    }}
    [data-theme="light"] {{
      --bg: #f8fafc;
      --card-bg: #ffffff;
      --text: #0f172a;
      --muted: #64748b;
      --accent: #0284c7;
      --border: #e2e8f0;
      --code-bg: #f1f5f9;
    }}

    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      background-color: var(--bg);
      color: var(--text);
      display: flex;
      flex-direction: column;
      height: 100vh;
      overflow: hidden;
      transition: background-color 0.2s, color 0.2s;
    }}

    /* Top Glass Navbar */
    header {{
      background: var(--card-bg);
      border-bottom: 1px solid var(--border);
      padding: 10px 20px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      user-select: none;
    }}
    .brand {{
      display: flex;
      align-items: center;
      gap: 10px;
      font-weight: 700;
      font-size: 1.1rem;
      color: var(--accent);
    }}
    .doc-meta {{
      display: flex;
      align-items: center;
      gap: 12px;
      font-size: 0.85rem;
    }}
    .badge {{
      background: var(--bg);
      border: 1px solid var(--border);
      padding: 4px 10px;
      border-radius: 999px;
      color: var(--muted);
    }}
    .badge strong {{ color: var(--accent); }}

    .controls {{
      display: flex;
      align-items: center;
      gap: 8px;
    }}
    select, button {{
      background: var(--bg);
      color: var(--text);
      border: 1px solid var(--border);
      padding: 6px 12px;
      border-radius: 6px;
      font-size: 0.85rem;
      cursor: pointer;
      outline: none;
    }}
    select:hover, button:hover {{
      border-color: var(--accent);
    }}

    /* Main Container */
    .app-body {{
      display: flex;
      flex: 1;
      overflow: hidden;
    }}

    /* TOC Sidebar */
    aside {{
      width: 280px;
      background: var(--card-bg);
      border-right: 1px solid var(--border);
      overflow-y: auto;
      padding: 16px 8px;
      display: flex;
      flex-direction: column;
      gap: 4px;
    }}
    aside h3 {{
      font-size: 0.8rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--muted);
      margin-bottom: 8px;
      padding-left: 8px;
    }}
    .toc-link {{
      color: var(--text);
      text-decoration: none;
      font-size: 0.88rem;
      padding: 6px 8px;
      border-radius: 4px;
      display: block;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }}
    .toc-link:hover {{
      background: var(--bg);
      color: var(--accent);
    }}

    /* Main Viewport */
    main {{
      flex: 1;
      overflow-y: auto;
      padding: 32px 48px;
      line-height: 1.7;
    }}
    .markdown-body {{
      max-width: 860px;
      margin: 0 auto;
    }}
    .markdown-body h1, .markdown-body h2, .markdown-body h3 {{
      margin-top: 1.5em;
      margin-bottom: 0.6em;
      color: var(--accent);
      border-bottom: 1px solid var(--border);
      padding-bottom: 0.3em;
    }}
    .markdown-body p {{ margin-bottom: 1em; }}
    .markdown-body code {{
      background: var(--code-bg);
      border: 1px solid var(--border);
      padding: 2px 6px;
      border-radius: 4px;
      font-family: monospace;
      font-size: 0.9em;
    }}
    .markdown-body pre {{
      background: var(--code-bg);
      border: 1px solid var(--border);
      padding: 16px;
      border-radius: 8px;
      overflow-x: auto;
      margin-bottom: 1em;
    }}
    .markdown-body pre code {{
      background: transparent;
      border: none;
      padding: 0;
    }}
    .markdown-body table {{
      width: 100%;
      border-collapse: collapse;
      margin-bottom: 1em;
    }}
    .markdown-body th, .markdown-body td {{
      border: 1px solid var(--border);
      padding: 8px 12px;
      text-align: left;
    }}
    .markdown-body th {{
      background: var(--card-bg);
      color: var(--accent);
    }}
    .markdown-body blockquote {{
      border-left: 4px solid var(--accent);
      padding-left: 16px;
      color: var(--muted);
      margin-bottom: 1em;
    }}
  </style>
</head>
<body>
  <header>
    <div class="brand">
      <span>🪶 Pankh Desktop Reader</span>
    </div>
    <div class="doc-meta">
      <span class="badge" title="{}">📄 <strong>{}</strong></span>
      <span class="badge">⚡ Tokens: <strong>{}</strong></span>
      <span class="badge">📝 Words: <strong>{}</strong></span>
    </div>
    <div class="controls">
      <button onclick="openFile()">📂 Open File</button>
      <select id="themeSelect" onchange="setTheme(this.value)">
        <option value="ocean">Ocean Dark 🌙</option>
        <option value="dracula">Dracula 🧛</option>
        <option value="gruvbox">Gruvbox 🌲</option>
        <option value="light">Clean Light ☀️</option>
      </select>
      <button onclick="toggleToc()">☰ TOC</button>
    </div>
  </header>

  <div class="app-body">
    <aside id="tocSidebar">
      <h3>Table of Contents</h3>
      {}
    </aside>

    <main>
      <div class="markdown-body">
        {}
      </div>
    </main>
  </div>

  <script>
    function setTheme(theme) {{
      document.documentElement.setAttribute('data-theme', theme);
    }}
    function toggleToc() {{
      const sidebar = document.getElementById('tocSidebar');
      sidebar.style.display = sidebar.style.display === 'none' ? 'flex' : 'none';
    }}
    function openFile() {{
      if (window.ipc) {{
        window.ipc.postMessage('open_file');
      }}
    }}
    document.addEventListener('keydown', function(e) {{
      if (e.ctrlKey && e.key === 'o') {{
        e.preventDefault();
        openFile();
      }}
    }});
  </script>
</body>
</html>"#,
        html_escape(&doc_title),
        html_escape(&path_str),
        html_escape(&doc_title),
        stats.estimated_tokens,
        stats.words,
        toc_html,
        rendered_body
    )
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Launches Pankh Native Desktop Reader powered by wry (zero-overhead OS webview)
pub fn run_gui(file_path: Option<&Path>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let event_loop = EventLoopBuilder::new().build();

    let initial_file = file_path.map(|p| p.to_path_buf());
    let initial_content = if let Some(ref path) = initial_file {
        read_markdown_file_safe(path)
            .unwrap_or_else(|_| "# Welcome to Pankh\nCould not open specified file.".to_string())
    } else {
        "# Welcome to Pankh Reader 🪶\n\nDrag & drop a Markdown file here or click **Open File** (Ctrl+O) to begin reading.".to_string()
    };

    let title = initial_file
        .as_ref()
        .map(|p| format!("Pankh Reader - {}", p.display()))
        .unwrap_or_else(|| "Pankh Desktop Reader".to_string());

    let window = WindowBuilder::new()
        .with_title(&title)
        .with_inner_size(tao::dpi::LogicalSize::new(1024.0, 768.0))
        .build(&event_loop)?;

    let html_content = generate_gui_html(&initial_content, initial_file.as_deref());

    let _webview = WebViewBuilder::new()
        .with_html(html_content)
        .with_ipc_handler(move |req: String| {
            if req == "open_file" {
                if let Some(picked_path) = rfd::FileDialog::new()
                    .add_filter("Markdown", &["md", "markdown"])
                    .pick_file()
                {
                    if let Ok(new_content) = read_markdown_file_safe(&picked_path) {
                        let _new_html = generate_gui_html(&new_content, Some(&picked_path));
                    }
                }
            }
        })
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {}
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_html() {
        let md = "# Title\n\nParagraph text with **bold**.";
        let html = markdown_to_html(md);
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_generate_gui_html() {
        let md = "# Architecture\n\nSystem overview.";
        let html = generate_gui_html(md, Some(Path::new("README.md")));
        assert!(html.contains("Pankh Reader - README.md"));
        assert!(html.contains("Architecture"));
        assert!(html.contains("Tokens:"));
    }
}
