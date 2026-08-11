use pulldown_cmark::{Options, Parser};

/// Parses raw Markdown input into pulldown-cmark event iterator with tables, task lists, and strikethrough enabled
pub fn parse_markdown(input: &str) -> Parser<'_> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    Parser::new_ext(input, options)
}
