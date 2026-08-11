use std::process::Command;

fn pankh_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pankh"))
}

#[test]
fn test_cli_agent_flag() {
    let output = pankh_cmd()
        .args(["tests/sample.md", "--agent"])
        .output()
        .expect("failed to run CLI");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Pankh Sample Document"));
    assert!(!stdout.contains("img.shields.io"));
}

#[test]
fn test_cli_outline_flag() {
    let output = pankh_cmd()
        .args(["tests/sample.md", "--outline"])
        .output()
        .expect("failed to run CLI");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Pankh Sample Document"));
    assert!(stdout.contains("Installation"));
}

#[test]
fn test_cli_code_flag() {
    let output = pankh_cmd()
        .args(["tests/sample.md", "--code", "--lang", "rust"])
        .output()
        .expect("failed to run CLI");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello Pankh!"));
    assert!(!stdout.contains("Hello Python"));
}

#[test]
fn test_cli_stats_flag() {
    let output = pankh_cmd()
        .args(["tests/sample.md", "--stats"])
        .output()
        .expect("failed to run CLI");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Lines"));
    assert!(stdout.contains("Estimated Tokens"));
    assert!(stdout.contains("Estimated LLM Input Cost"));
}

#[test]
fn test_cli_max_tokens_flag() {
    let output = pankh_cmd()
        .args(["tests/sample.md", "--max-tokens", "15"])
        .output()
        .expect("failed to run CLI");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Chunk 1"));
}

#[test]
fn test_cli_diff_clean_flag() {
    let output = pankh_cmd()
        .args(["tests/sample.md", "--diff-clean"])
        .output()
        .expect("failed to run CLI");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PANKH TOKEN OPTIMIZATION DIFF REPORT"));
    assert!(stdout.contains("Tokens Saved"));
    assert!(stdout.contains("Estimated Dollar Savings"));
}

#[test]
fn test_cli_search_flag() {
    let output = pankh_cmd()
        .args(["tests/sample.md", "--search", "Installation"])
        .output()
        .expect("failed to run CLI");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Search Results for \"Installation\""));
    assert!(stdout.contains("H2: Installation"));
    assert!(stdout.contains("Score:"));
}

#[test]
fn test_cli_llmstxt_flag() {
    let output = pankh_cmd()
        .args(["tests/sample.md", "--llms-txt", "--json"])
        .output()
        .expect("failed to run CLI");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("index_content"));
    assert!(stdout.contains("full_content"));
}

#[test]
fn test_cli_build_index_flag() {
    let output = pankh_cmd()
        .args(["tests/sample.md", "--build-index"])
        .output()
        .expect("failed to run CLI");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Successfully saved pre-computed search index"));

    let _ = std::fs::remove_file(pankh::core::index::DEFAULT_INDEX_FILENAME);
}
