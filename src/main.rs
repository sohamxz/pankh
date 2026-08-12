use clap::Parser;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub mod core;
pub mod gui;
pub mod mcp;
pub mod tui;

use core::agent::{
    calculate_stats, clean_markdown, extract_code_blocks, extract_outline, generate_clean_diff,
    HeadingNode,
};
use core::chunker::chunk_markdown;
use core::io::read_markdown_file_safe;
use core::llmstxt::{generate_llmstxt, write_llmstxt_to_dir};
use core::search::search_documents;

#[derive(Parser, Debug)]
#[command(
    name = "pankh",
    author,
    version,
    about = "Featherweight Markdown Reader for Humans & AI Agents"
)]
pub struct Cli {
    /// Path to markdown file(s) or directory (reads stdin if omitted)
    pub files: Vec<PathBuf>,

    /// Agent mode: Output token-thrifty clean markdown to stdout
    #[arg(short, long)]
    pub agent: bool,

    /// Output structural outline tree of headings
    #[arg(short, long)]
    pub outline: bool,

    /// Extract code snippets only
    #[arg(short, long)]
    pub code: bool,

    /// Filter extracted code snippets by language extension (e.g. rs, python)
    #[arg(long)]
    pub lang: Option<String>,

    /// Search for query term across markdown file(s) or directory
    #[arg(short = 'S', long)]
    pub search: Option<String>,

    /// Build pre-computed search index (.pankh_index.bin) for instant sub-5ms search
    #[arg(long)]
    pub build_index: bool,

    /// Override custom LLM price per 1M input tokens (e.g. --price-per-m 2.50)
    #[arg(long)]
    pub price_per_m: Option<f64>,

    /// Watch file(s) or directory for changes and live-reload TUI or auto-regenerate llms.txt
    #[arg(short, long)]
    pub watch: bool,

    /// Generate standard llms.txt index and llms-full.txt clean payload
    #[arg(long)]
    pub llms_txt: bool,

    /// Chunk document at heading boundaries up to target token budget
    #[arg(long)]
    pub max_tokens: Option<usize>,

    /// Display diff comparing raw markdown with cleaned token-thrifty output
    #[arg(long)]
    pub diff_clean: bool,

    /// Display stats: word count, line count, estimated token count
    #[arg(short, long)]
    pub stats: bool,

    /// Format outline/stats/chunks/search/llms-txt output as JSON
    #[arg(long)]
    pub json: bool,

    /// Run as stdio MCP (Model Context Protocol) server
    #[arg(long)]
    pub mcp: bool,

    /// Launch optional featherweight native desktop GUI interface
    #[arg(short = 'g', long)]
    pub gui: bool,
}

fn collect_markdown_files(path: &Path, acc: &mut Vec<PathBuf>) {
    if path.is_file() {
        if let Some(ext) = path.extension() {
            if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown") {
                acc.push(path.to_path_buf());
            }
        }
    } else if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                collect_markdown_files(&entry_path, acc);
            }
        }
    }
}

fn print_heading_tree_node(node: &HeadingNode) {
    let indent = "  ".repeat(node.level.saturating_sub(1) as usize);
    println!(
        "{}- H{}: {} [L{}-{}, Tokens: {}, Chars: {}]",
        indent,
        node.level,
        node.title,
        node.start_line,
        node.end_line,
        node.token_count,
        node.character_count
    );
    for child in &node.children {
        print_heading_tree_node(child);
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.gui {
        let target_file = cli.files.first().map(|p| p.as_path());
        gui::run_gui(target_file).map_err(|e| anyhow::anyhow!("{}", e))?;
        return Ok(());
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async_main(cli))
}

async fn async_main(cli: Cli) -> anyhow::Result<()> {
    if cli.mcp {
        return mcp::server::run_mcp_server().await;
    }

    // Resolve file targets (recursively expands directories)
    let mut resolved_files = Vec::new();
    for f in &cli.files {
        collect_markdown_files(f, &mut resolved_files);
    }

    // Mode: --build-index flag
    if cli.build_index {
        let targets = if resolved_files.is_empty() && !cli.files.is_empty() {
            cli.files.clone()
        } else if resolved_files.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            resolved_files.clone()
        };

        println!(
            "Building pre-computed search index across {} files...",
            targets.len()
        );
        let index = core::index::build_search_index(&targets);
        let index_path = PathBuf::from(core::index::DEFAULT_INDEX_FILENAME);
        core::index::save_index_to_file(&index, &index_path)?;
        println!(
            "Successfully saved pre-computed search index to '{}' ({} docs indexed, {} term postings)",
            index_path.display(), index.docs.len(), index.term_posting.len()
        );
        return Ok(());
    }

    // Mode 1: --search flag
    if let Some(ref query) = cli.search {
        let index_path = PathBuf::from(core::index::DEFAULT_INDEX_FILENAME);
        let result = if index_path.exists() {
            if let Ok(loaded_index) = core::index::load_index_from_file(&index_path) {
                core::index::search_with_index(&loaded_index, query)
            } else {
                let targets = if resolved_files.is_empty() && !cli.files.is_empty() {
                    cli.files.clone()
                } else if resolved_files.is_empty() {
                    vec![PathBuf::from(".")]
                } else {
                    resolved_files.clone()
                };
                search_documents(&targets, query)
            }
        } else {
            let targets = if resolved_files.is_empty() && !cli.files.is_empty() {
                cli.files.clone()
            } else if resolved_files.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                resolved_files.clone()
            };
            search_documents(&targets, query)
        };

        if cli.json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!(
                "Search Results for \"{}\" ({} hits across {} files searched):",
                result.query, result.total_hits, result.files_searched
            );
            for hit in &result.hits {
                println!(
                    "- [{}:{}] ({}) [Score: {:.2}]\n  > {} (Section Tokens: {})",
                    hit.file_path,
                    hit.line_number,
                    hit.heading_path,
                    hit.score,
                    hit.line_snippet,
                    hit.section_tokens
                );
            }
        }
        return Ok(());
    }

    // Mode 2: --llms-txt flag
    if cli.llms_txt {
        let targets = if resolved_files.is_empty() && !cli.files.is_empty() {
            cli.files.clone()
        } else if resolved_files.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            resolved_files.clone()
        };

        let result = core::llmstxt::generate_llmstxt(&targets);
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            std::fs::write(
                core::llmstxt::DEFAULT_LLMS_TXT_FILENAME,
                &result.index_content,
            )?;
            std::fs::write(
                core::llmstxt::DEFAULT_LLMS_FULL_TXT_FILENAME,
                &result.full_content,
            )?;
            println!(
                "Successfully generated llms.txt & llms-full.txt across {} files!\n- Raw Tokens: {}\n- Clean Payload Tokens: {}\n- Tokens Saved: {}",
                result.files_processed, result.total_raw_tokens, result.total_clean_tokens, result.tokens_saved
            );
        }

        if cli.watch {
            let watch_targets = if cli.files.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                cli.files.clone()
            };

            let watch_targets_closure = watch_targets.clone();

            println!("\n[Watcher Active] Watching for changes to auto-regenerate llms.txt & llms-full.txt...");
            let _ = core::watcher::watch_paths(&watch_targets, move || {
                let res = core::llmstxt::generate_llmstxt(&watch_targets_closure);
                let _ =
                    std::fs::write(core::llmstxt::DEFAULT_LLMS_TXT_FILENAME, &res.index_content);
                let _ = std::fs::write(
                    core::llmstxt::DEFAULT_LLMS_FULL_TXT_FILENAME,
                    &res.full_content,
                );
                println!(
                    "[Watcher] Auto-regenerated llms.txt & llms-full.txt across {} files (Saved {} tokens)",
                    res.files_processed, res.tokens_saved
                );
            });

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }

        return Ok(());
    }

    // Read input from resolved file(s) or stdin
    let mut raw_content = String::new();
    if resolved_files.is_empty() {
        if cli.files.is_empty() {
            io::stdin().read_to_string(&mut raw_content)?;
        } else {
            for path in &cli.files {
                match read_markdown_file_safe(path) {
                    Ok(file_text) => raw_content.push_str(&file_text),
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    } else {
        for (i, path) in resolved_files.iter().enumerate() {
            match read_markdown_file_safe(path) {
                Ok(file_text) => {
                    if i > 0 && !raw_content.is_empty() {
                        raw_content.push_str("\n\n---\n\n");
                    }
                    raw_content.push_str(&file_text);
                }
                Err(e) => {
                    eprintln!("Error: Skipping '{}': {}", path.display(), e);
                }
            }
        }
    }

    // Mode 3: --diff-clean flag
    if cli.diff_clean {
        let diff = generate_clean_diff(&raw_content);
        println!("{}", diff);
        return Ok(());
    }

    // Mode 4: --max-tokens flag
    if let Some(limit) = cli.max_tokens {
        let chunks = chunk_markdown(&raw_content, limit);
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&chunks)?);
        } else {
            for chunk in &chunks {
                println!(
                    "--- Chunk {} ({}) [Lines {}-{}, Tokens: {}] ---\n{}\n",
                    chunk.chunk_index,
                    chunk.title,
                    chunk.start_line,
                    chunk.end_line,
                    chunk.token_count,
                    chunk.content
                );
            }
        }
        return Ok(());
    }

    // Mode 5: --agent flag
    if cli.agent {
        let cleaned = clean_markdown(&raw_content);
        println!("{}", cleaned);
        return Ok(());
    }

    // Mode 6: --outline flag
    if cli.outline {
        let outline = extract_outline(&raw_content);
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&outline)?);
        } else {
            println!(
                "Outline (Total Sections: {}, Max Depth: H{}):",
                outline.total_sections, outline.max_depth
            );
            for heading in &outline.headings {
                print_heading_tree_node(heading);
            }
        }
        return Ok(());
    }

    // Mode 7: --llms-txt flag
    if cli.llms_txt {
        let targets = if resolved_files.is_empty() && !cli.files.is_empty() {
            cli.files.clone()
        } else if resolved_files.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            resolved_files.clone()
        };

        let output = generate_llmstxt(&targets);
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            let target_dir = PathBuf::from(".");
            match write_llmstxt_to_dir(&output, &target_dir) {
                Ok((idx_path, full_path)) => {
                    println!(
                        "Successfully generated llms.txt ('{}') and llms-full.txt ('{}')",
                        idx_path.display(),
                        full_path.display()
                    );
                    println!(
                        "Processed {} files | Raw Tokens: {} | Clean Tokens: {} | Tokens Saved: {}",
                        output.files_processed,
                        output.total_raw_tokens,
                        output.total_clean_tokens,
                        output.tokens_saved
                    );
                }
                Err(e) => {
                    eprintln!("Error saving llms.txt: {}", e);
                }
            }
        }
        return Ok(());
    }

    // Mode 7: --code flag
    if cli.code {
        let code_blocks = extract_code_blocks(&raw_content, cli.lang.as_deref());
        for (i, block) in code_blocks.iter().enumerate() {
            if i > 0 {
                println!("\n---");
            }
            println!("// Language: {}\n{}", block.language, block.code);
        }
        return Ok(());
    }

    // Mode 8: --stats flag
    if cli.stats {
        let stats = calculate_stats(&raw_content);
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&stats)?);
        } else {
            println!("Document Stats:");
            println!("- Lines: {}", stats.lines);
            println!("- Words: {}", stats.words);
            println!("- Characters: {}", stats.characters);
            println!("- Estimated Tokens: {}", stats.estimated_tokens);
            let costs = core::pricing::estimate_costs(
                stats.estimated_tokens,
                stats.estimated_tokens,
                cli.price_per_m,
            );
            println!("- Estimated LLM Input Cost:");
            for cost in costs {
                println!("  - {}: ${:.4}", cost.model_name, cost.raw_cost);
            }
        }
        return Ok(());
    }

    // Mode 9: Interactive TUI Reader (Human mode)
    let watch_targets = if resolved_files.is_empty() {
        cli.files.clone()
    } else {
        resolved_files
    };

    tui::app::run_tui(&raw_content, &watch_targets, cli.watch)?;

    Ok(())
}
