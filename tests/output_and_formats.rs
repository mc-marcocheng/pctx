//! Output rendering: formats, tree, dry-run, stats, output-to-file, JSON surface, injection safety.

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod common;
use common::{pctx, setup_test_project};

#[test]
fn test_basic_output() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("```rust"))
        .stdout(predicate::str::contains("Hello, world!"));
}

#[test]
fn test_json_output() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status": "success""#))
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn test_xml_format() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["--format", "xml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<?xml version=\"1.0\""))
        .stdout(predicate::str::contains("<context>"))
        .stdout(predicate::str::contains("<file path=\""))
        .stdout(predicate::str::contains("<![CDATA["));
}

#[test]
fn test_plain_format() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["--format", "plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== "))
        .stdout(predicate::str::contains("main.rs ==="));
}

#[test]
fn test_tree_in_output() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .arg("--tree")
        .assert()
        .success()
        .stdout(predicate::str::contains("## File Tree"))
        .stdout(predicate::str::contains("```"));
}

#[test]
fn test_tree_in_xml_format() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["--format", "xml", "--tree"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<tree>"))
        .stdout(predicate::str::contains("<![CDATA["))
        .stdout(predicate::str::contains("src"));
}

#[test]
fn test_tree_in_plain_format() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["--format", "plain", "--tree"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== File Tree ==="))
        .stdout(predicate::str::contains("src"));
}

#[test]
fn test_dry_run() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .arg("--dry-run")
        .assert()
        .success()
        // Dry run output goes to stderr
        .stderr(predicate::str::contains("Dry run"))
        .stderr(predicate::str::contains("main.rs"));
}

#[test]
fn test_dry_run_json() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["--dry-run", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"#))
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn test_dry_run_shows_tokens() {
    let dir = setup_test_project();

    // Dry run should show token estimate
    pctx()
        .current_dir(dir.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stderr(predicate::str::contains("tokens"));
}

#[test]
fn test_stats_flag() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .arg("--stats")
        .assert()
        .success()
        .stderr(predicate::str::contains("Statistics"))
        .stderr(predicate::str::contains("Files:"));
}

#[test]
fn test_output_to_file() {
    let dir = setup_test_project();
    let output_file = dir.path().join("output.md");

    pctx()
        .current_dir(dir.path())
        .args(["--output", "output.md", "--exclude", "output.md"])
        .assert()
        .success();

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("main.rs"));
}

#[test]
fn test_output_file_exists_error() {
    let dir = setup_test_project();
    let output_file = dir.path().join("output.md");

    // Create the file first
    fs::write(&output_file, "existing content").unwrap();

    pctx()
        .current_dir(dir.path())
        .args(["--output", "output.md"])
        .assert()
        .code(5) // CONFLICT exit code
        .stderr(predicate::str::contains("exists"));
}

#[test]
fn test_output_file_force_overwrite() {
    let dir = setup_test_project();
    let output_file = dir.path().join("output.md");

    // Create the file first
    fs::write(&output_file, "existing content").unwrap();

    pctx()
        .current_dir(dir.path())
        .args(["--output", "output.md", "--force", "--exclude", "output.md"])
        .assert()
        .success();

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("main.rs"));
    assert!(!content.contains("existing content"));
}

#[test]
fn test_json_error_format() {
    pctx()
        .args(["--json", "/nonexistent/path"])
        .assert()
        .code(3)
        .stdout(predicate::str::contains(r#""status": "error""#))
        .stdout(predicate::str::contains(r#""code":"#))
        .stdout(predicate::str::contains(r#""suggestion":"#));
}

#[test]
fn test_json_line_count_omitted_for_file_list() {
    let dir = setup_test_project();

    let output = pctx()
        .current_dir(dir.path())
        .args(["files", "list", "--json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // line_count should be omitted when 0 (not calculated)
    // The JSON should not contain "line_count": 0
    assert!(!stdout.contains(r#""line_count": 0"#));
}

#[test]
fn test_cdata_injection_prevention() {
    let dir = TempDir::new().unwrap();

    // Create a file with CDATA closing sequence
    fs::write(
        dir.path().join("evil.txt"),
        "Some content ]]> with CDATA injection attempt",
    )
    .unwrap();

    let output = pctx()
        .current_dir(dir.path())
        .args(["--format", "xml"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    // The raw ]]> should not appear unescaped in CDATA
    // It should be escaped as ]]]]><![CDATA[>
    assert!(!stdout.contains("<![CDATA[Some content ]]>") || stdout.contains("]]]]><![CDATA[>"));
}

#[test]
fn test_fence_injection_prevention() {
    let dir = TempDir::new().unwrap();

    // Create a file with triple backticks in content
    fs::write(
        dir.path().join("code.md"),
        "Some text\n```\ncode block\n```\nmore text",
    )
    .unwrap();

    let output = pctx().current_dir(dir.path()).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Should use 4 backticks for the fence since content has 3
    assert!(stdout.contains("````markdown"));
    assert!(stdout.contains("````"));
}

#[test]
fn test_fence_injection_with_four_backticks() {
    let dir = TempDir::new().unwrap();

    // Create a file with four backticks in content
    fs::write(
        dir.path().join("code.md"),
        "Some text\n````\ncode block\n````\nmore text",
    )
    .unwrap();

    let output = pctx().current_dir(dir.path()).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Should use 5 backticks for the fence since content has 4
    assert!(stdout.contains("`````markdown"));
    assert!(stdout.contains("`````"));
}

#[test]
fn test_line_truncation_no_extra_newlines() {
    let dir = TempDir::new().unwrap();

    // Create a file with exactly enough lines to trigger truncation
    let content: String = (1..=50)
        .map(|i| format!("line {}", i))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(dir.path().join("test.txt"), &content).unwrap();

    let output = pctx()
        .current_dir(dir.path())
        .args([
            "--max-lines",
            "10",
            "--head-lines",
            "3",
            "--tail-lines",
            "2",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Should not have multiple consecutive newlines around the truncation marker
    assert!(!stdout.contains("\n\n\n"));
}
