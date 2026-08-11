use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::core::agent::{calculate_stats, clean_markdown, extract_code_blocks, extract_outline};
use crate::core::chunker::chunk_markdown;
use crate::core::index::{
    build_search_index, load_index_from_file, save_index_to_file, search_with_index, SearchIndex,
    DEFAULT_INDEX_FILENAME,
};
use crate::core::io::read_markdown_file_safe;
use crate::core::watcher::watch_paths;
use crate::tui::app::flatten_headings;

#[derive(Clone)]
pub struct ServerState {
    pub index: Arc<RwLock<SearchIndex>>,
    pub workspace_root: PathBuf,
}

impl ServerState {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        let path = root.as_ref().to_path_buf();
        let index_path = path.join(DEFAULT_INDEX_FILENAME);
        let index = if index_path.exists() {
            load_index_from_file(&index_path)
                .unwrap_or_else(|_| build_search_index(std::slice::from_ref(&path)))
        } else {
            build_search_index(std::slice::from_ref(&path))
        };

        ServerState {
            index: Arc::new(RwLock::new(index)),
            workspace_root: path,
        }
    }
}

/// Standard JSON-RPC 2.0 Error response builder
fn build_jsonrpc_error(id: Option<Value>, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

/// Validates and canonicalizes a file path safely to prevent path traversal issues
pub fn validate_mcp_path(requested: &str) -> Result<PathBuf, String> {
    let trimmed = requested
        .trim_start_matches("file:///")
        .trim_start_matches("file://");
    if trimmed.trim().is_empty() {
        return Err("Path argument cannot be empty".to_string());
    }

    let path = Path::new(trimmed);
    match fs::canonicalize(path) {
        Ok(canonical) => Ok(canonical),
        Err(_) => Err(format!("File not found or invalid path: {}", requested)),
    }
}

/// Handles an incoming JSON-RPC message string using default ServerState
pub async fn handle_jsonrpc_message(raw: &str) -> anyhow::Result<Option<String>> {
    let state = ServerState::new(".");
    handle_jsonrpc_message_with_state(raw, &state).await
}

/// Handles an incoming JSON-RPC message string with a given ServerState
pub async fn handle_jsonrpc_message_with_state(
    raw: &str,
    state: &ServerState,
) -> anyhow::Result<Option<String>> {
    let msg: Value = match serde_json::from_str(raw) {
        Ok(val) => val,
        Err(_) => {
            return Ok(Some(
                build_jsonrpc_error(None, -32700, "Parse error").to_string(),
            ))
        }
    };

    let id = msg.get("id").cloned();
    let method = match msg.get("method").and_then(|m| m.as_str()) {
        Some(m) => m,
        None => {
            return Ok(Some(
                build_jsonrpc_error(id, -32600, "Invalid Request").to_string(),
            ))
        }
    };

    // Notification silencing for notifications/... and $/... (e.g. $/cancelRequest)
    if method.starts_with("$/") || method.starts_with("notifications/") {
        return Ok(None);
    }

    let response = match method {
        "initialize" => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {},
                        "resources": {},
                        "prompts": {}
                    },
                    "serverInfo": {
                        "name": "pankh",
                        "version": "0.1.0"
                    }
                }
            })
        }
        "prompts/list" => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "prompts": [
                        {
                            "name": "summarize_markdown",
                            "description": "Returns a clean, badge-free summary of a Markdown document",
                            "arguments": [
                                { "name": "path", "description": "Path to Markdown file", "required": true }
                            ]
                        },
                        {
                            "name": "extract_architecture_decisions",
                            "description": "Extracts heading architecture outline and code blocks from Markdown doc",
                            "arguments": [
                                { "name": "path", "description": "Path to Markdown file", "required": true }
                            ]
                        }
                    ]
                }
            })
        }
        "prompts/get" => {
            let params = msg.get("params").cloned().unwrap_or(Value::Null);
            let prompt_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
            let raw_path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");

            let canonical_path = match validate_mcp_path(raw_path) {
                Ok(p) => p,
                Err(err_msg) => {
                    return Ok(Some(build_jsonrpc_error(id, -32602, &err_msg).to_string()))
                }
            };

            let file_content = match read_markdown_file_safe(&canonical_path) {
                Ok(c) => c,
                Err(e) => {
                    return Ok(Some(
                        build_jsonrpc_error(id, -32602, &e.to_string()).to_string(),
                    ))
                }
            };

            let cleaned = clean_markdown(&file_content);

            let prompt_text = match prompt_name {
                "summarize_markdown" => format!(
                    "Please provide a concise summary of the following Markdown document:\n\n{}",
                    cleaned
                ),
                "extract_architecture_decisions" => {
                    let outline = extract_outline(&file_content);
                    let code_blocks = extract_code_blocks(&file_content, None);
                    format!(
                        "Analyze the architecture decisions from this outline:\n\nTree: {}\n\nCode Snippets Count: {}",
                        serde_json::to_string_pretty(&outline).unwrap_or_default(),
                        code_blocks.len()
                    )
                }
                _ => {
                    return Ok(Some(
                        build_jsonrpc_error(
                            id,
                            -32602,
                            &format!("Unknown prompt: {}", prompt_name),
                        )
                        .to_string(),
                    ))
                }
            };

            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "description": format!("Prompt response for {}", prompt_name),
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": prompt_text
                            }
                        }
                    ]
                }
            })
        }
        "resources/list" => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "resources": [
                        {
                            "uri": "file:///path/to/markdown.md",
                            "name": "Local Markdown Document",
                            "mimeType": "text/markdown",
                            "description": "Access local markdown documents as token-optimized context"
                        }
                    ]
                }
            })
        }
        "resources/read" => {
            let params = msg.get("params").cloned().unwrap_or(Value::Null);
            let uri = params.get("uri").and_then(|u| u.as_str()).unwrap_or("");
            let canonical_path = match validate_mcp_path(uri) {
                Ok(p) => p,
                Err(err_msg) => {
                    return Ok(Some(build_jsonrpc_error(id, -32602, &err_msg).to_string()))
                }
            };

            let file_content = match read_markdown_file_safe(&canonical_path) {
                Ok(c) => c,
                Err(e) => {
                    return Ok(Some(
                        build_jsonrpc_error(id, -32602, &e.to_string()).to_string(),
                    ))
                }
            };

            let cleaned = clean_markdown(&file_content);

            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": "text/markdown",
                            "text": cleaned
                        }
                    ]
                }
            })
        }
        "tools/list" => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "read_clean_markdown",
                            "description": "Reads a markdown file and returns token-thrifty clean text stripped of visual badges and clutter",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "Path to markdown file" }
                                },
                                "required": ["path"]
                            }
                        },
                        {
                            "name": "get_markdown_outline",
                            "description": "Extracts nested heading hierarchy tree (H1-H6) with line numbers and token counts from markdown file",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "Path to markdown file" }
                                },
                                "required": ["path"]
                            }
                        },
                        {
                            "name": "read_markdown_section",
                            "description": "Reads a specific section of markdown under a given heading title",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "Path to markdown file" },
                                    "heading": { "type": "string", "description": "Header title to extract" }
                                },
                                "required": ["path", "heading"]
                            }
                        },
                        {
                            "name": "extract_code_blocks",
                            "description": "Extracts code blocks from markdown file, optionally filtered by language",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "Path to markdown file" },
                                    "lang": { "type": "string", "description": "Optional language extension filter" }
                                },
                                "required": ["path"]
                            }
                        },
                        {
                            "name": "search_markdown_sections",
                            "description": "Searches sections of a markdown document for a query string",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "Path to markdown file" },
                                    "query": { "type": "string", "description": "Query term to search" }
                                },
                                "required": ["path", "query"]
                            }
                        },
                        {
                            "name": "chunk_markdown",
                            "description": "Splits a markdown document into heading-aware chunks under max_tokens budget",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "Path to markdown file" },
                                    "max_tokens": { "type": "number", "description": "Maximum token budget per chunk" }
                                },
                                "required": ["path", "max_tokens"]
                            }
                        },
                        {
                            "name": "estimate_tokens",
                            "description": "Estimates LLM token count and line/word statistics for a markdown file",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "Path to markdown file" }
                                },
                                "required": ["path"]
                            }
                        }
                    ]
                }
            })
        }
        "tools/call" => {
            let params = msg.get("params").cloned().unwrap_or(Value::Null);
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);

            let raw_path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");

            let result_text = match name {
                "search_markdown_sections" => {
                    let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
                    if !raw_path.trim().is_empty() {
                        let canonical_path = match validate_mcp_path(raw_path) {
                            Ok(p) => p,
                            Err(err_msg) => {
                                return Ok(Some(
                                    build_jsonrpc_error(id, -32602, &err_msg).to_string(),
                                ))
                            }
                        };
                        let file_content = match read_markdown_file_safe(&canonical_path) {
                            Ok(c) => c,
                            Err(e) => {
                                return Ok(Some(
                                    build_jsonrpc_error(id, -32602, &e.to_string()).to_string(),
                                ))
                            }
                        };
                        let outline = extract_outline(&file_content);
                        let flat_headings = flatten_headings(&outline.headings);
                        let matching_headings: Vec<_> = flat_headings
                            .iter()
                            .filter(|h| h.title.to_lowercase().contains(&query.to_lowercase()))
                            .collect();
                        serde_json::to_string_pretty(&matching_headings).unwrap_or_default()
                    } else {
                        let guard = state.index.read().unwrap_or_else(|e| e.into_inner());
                        let search_res = search_with_index(&guard, query);
                        serde_json::to_string_pretty(&search_res).unwrap_or_default()
                    }
                }
                _ => {
                    let canonical_path = match validate_mcp_path(raw_path) {
                        Ok(p) => p,
                        Err(err_msg) => {
                            return Ok(Some(build_jsonrpc_error(id, -32602, &err_msg).to_string()))
                        }
                    };

                    let file_content = match read_markdown_file_safe(&canonical_path) {
                        Ok(c) => c,
                        Err(e) => {
                            return Ok(Some(
                                build_jsonrpc_error(id, -32602, &e.to_string()).to_string(),
                            ))
                        }
                    };

                    match name {
                        "read_clean_markdown" => clean_markdown(&file_content),
                        "get_markdown_outline" => {
                            let outline = extract_outline(&file_content);
                            serde_json::to_string_pretty(&outline).unwrap_or_default()
                        }
                        "read_markdown_section" => {
                            let target_heading = args
                                .get("heading")
                                .and_then(|h| h.as_str())
                                .unwrap_or("")
                                .to_lowercase();
                            let outline = extract_outline(&file_content);
                            let flat_headings = flatten_headings(&outline.headings);
                            let mut section_lines = Vec::new();
                            let mut recording = false;

                            let target_line = flat_headings
                                .iter()
                                .find(|h| h.title.to_lowercase().contains(&target_heading));

                            if let Some(target) = target_line {
                                for (idx, line) in file_content.lines().enumerate() {
                                    let line_num = idx + 1;
                                    if line_num == target.start_line {
                                        recording = true;
                                    } else if recording && line_num > target.end_line {
                                        break;
                                    }
                                    if recording {
                                        section_lines.push(line);
                                    }
                                }
                            }
                            section_lines.join("\n")
                        }
                        "extract_code_blocks" => {
                            let lang = args.get("lang").and_then(|l| l.as_str());
                            let code_blocks = extract_code_blocks(&file_content, lang);
                            let mut output = String::new();
                            for (i, block) in code_blocks.iter().enumerate() {
                                if i > 0 {
                                    output.push_str("\n---\n");
                                }
                                output.push_str(&format!(
                                    "// Lang: {}\n{}",
                                    block.language, block.code
                                ));
                            }
                            output
                        }
                        "chunk_markdown" => {
                            let max_tokens =
                                args.get("max_tokens")
                                    .and_then(|m| m.as_u64())
                                    .unwrap_or(500) as usize;
                            let chunks = chunk_markdown(&file_content, max_tokens);
                            serde_json::to_string_pretty(&chunks).unwrap_or_default()
                        }
                        "estimate_tokens" => {
                            let stats = calculate_stats(&file_content);
                            serde_json::to_string_pretty(&stats).unwrap_or_default()
                        }
                        _ => {
                            return Ok(Some(
                                build_jsonrpc_error(id, -32601, &format!("Unknown tool: {}", name))
                                    .to_string(),
                            ))
                        }
                    }
                }
            };

            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": result_text
                        }
                    ]
                }
            })
        }
        _ => build_jsonrpc_error(id, -32601, "Method not found"),
    };

    Ok(Some(response.to_string()))
}

/// Runs stdio MCP server loop with real-time background auto-indexing daemon
pub async fn run_mcp_server() -> anyhow::Result<()> {
    let state = ServerState::new(".");
    let index_clone = Arc::clone(&state.index);
    let root_clone = state.workspace_root.clone();

    // Spawn real-time background auto-indexing daemon thread
    std::thread::spawn(move || {
        let targets = vec![root_clone.clone()];
        let watch_targets_closure = targets.clone();
        let _ = watch_paths(&targets, move || {
            let mut guard = index_clone.write().unwrap_or_else(|e| e.into_inner());
            let fresh = build_search_index(&watch_targets_closure);
            *guard = fresh;
            let _ = save_index_to_file(&guard, &root_clone.join(DEFAULT_INDEX_FILENAME));
            let notif = json!({
                "jsonrpc": "2.0",
                "method": "notifications/resources/updated",
                "params": {
                    "uri": format!("file://{}", root_clone.display())
                }
            });
            println!("{}", notif);
        });
    });

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    while reader.read_line(&mut line).await? > 0 {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if let Ok(Some(resp)) = handle_jsonrpc_message_with_state(trimmed, &state).await {
                stdout.write_all(resp.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
        }
        line.clear();
    }

    Ok(())
}
