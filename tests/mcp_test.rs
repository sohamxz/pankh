use pankh::mcp::server::handle_jsonrpc_message;

#[tokio::test]
async fn test_mcp_initialize() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let res = handle_jsonrpc_message(req).await.unwrap().unwrap();
    assert!(res.contains("serverInfo"));
    assert!(res.contains("pankh"));
}

#[tokio::test]
async fn test_mcp_notification_silencing() {
    let cancel_req = r#"{"jsonrpc":"2.0","method":"$/cancelRequest","params":{"requestId":1}}"#;
    let res = handle_jsonrpc_message(cancel_req).await.unwrap();
    assert!(res.is_none());

    let init_notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}"#;
    let res2 = handle_jsonrpc_message(init_notif).await.unwrap();
    assert!(res2.is_none());
}

#[tokio::test]
async fn test_mcp_tools_list() {
    let req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
    let res = handle_jsonrpc_message(req).await.unwrap().unwrap();
    assert!(res.contains("read_clean_markdown"));
    assert!(res.contains("get_markdown_outline"));
    assert!(res.contains("search_markdown_sections"));
    assert!(res.contains("chunk_markdown"));
    assert!(res.contains("estimate_tokens"));
}

#[tokio::test]
async fn test_mcp_prompts_list_and_get() {
    let req = r#"{"jsonrpc":"2.0","id":3,"method":"prompts/list","params":{}}"#;
    let res = handle_jsonrpc_message(req).await.unwrap().unwrap();
    assert!(res.contains("summarize_markdown"));
    assert!(res.contains("extract_architecture_decisions"));

    let get_req = r#"{"jsonrpc":"2.0","id":4,"method":"prompts/get","params":{"name":"summarize_markdown","arguments":{"path":"tests/sample.md"}}}"#;
    let get_res = handle_jsonrpc_message(get_req).await.unwrap().unwrap();
    assert!(get_res.contains("Pankh Sample Document"));
}

#[tokio::test]
async fn test_mcp_tool_call_read_clean() {
    let req = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"read_clean_markdown","arguments":{"path":"tests/sample.md"}}}"#;
    let res = handle_jsonrpc_message(req).await.unwrap().unwrap();
    assert!(res.contains("Pankh Sample Document"));
    assert!(!res.contains("img.shields.io"));
}

#[tokio::test]
async fn test_mcp_tool_call_search_sections() {
    let req = r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"search_markdown_sections","arguments":{"path":"tests/sample.md","query":"Installation"}}}"#;
    let res = handle_jsonrpc_message(req).await.unwrap().unwrap();
    assert!(res.contains("Installation"));
}

#[tokio::test]
async fn test_mcp_tool_call_chunk_markdown() {
    let req = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"chunk_markdown","arguments":{"path":"tests/sample.md","max_tokens":20}}}"#;
    let res = handle_jsonrpc_message(req).await.unwrap().unwrap();
    assert!(res.contains("chunk_index"));
}

#[tokio::test]
async fn test_mcp_resources_list() {
    let req = r#"{"jsonrpc":"2.0","id":8,"method":"resources/list","params":{}}"#;
    let res = handle_jsonrpc_message(req).await.unwrap().unwrap();
    assert!(res.contains("resources"));
    assert!(res.contains("file://"));
}

#[tokio::test]
async fn test_mcp_error_handling() {
    let invalid_json = "invalid json {";
    let res = handle_jsonrpc_message(invalid_json).await.unwrap().unwrap();
    assert!(res.contains("-32700")); // Parse error

    let missing_path = r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"read_clean_markdown","arguments":{}}}"#;
    let res2 = handle_jsonrpc_message(missing_path).await.unwrap().unwrap();
    assert!(res2.contains("-32602")); // Invalid params
}

#[tokio::test]
async fn test_mcp_server_state_auto_indexing() {
    let temp_dir = std::env::temp_dir().join("pankh_mcp_daemon_test");
    let _ = std::fs::create_dir_all(&temp_dir);
    let file1 = temp_dir.join("doc1.md");
    std::fs::write(&file1, "# Dynamic Title\nDynamic content text").unwrap();

    let state = pankh::mcp::server::ServerState::new(&temp_dir);
    let req = r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"search_markdown_sections","arguments":{"query":"Dynamic"}}}"#;
    let res = pankh::mcp::server::handle_jsonrpc_message_with_state(req, &state)
        .await
        .unwrap()
        .unwrap();
    assert!(res.contains("doc1.md") || res.contains("Dynamic"));

    let _ = std::fs::remove_dir_all(temp_dir);
}
