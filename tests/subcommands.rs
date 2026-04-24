//! Subcommands (files, config, completions), --stdin mode, and top-level CLI surface.

use predicates::prelude::*;
use std::fs;
use std::path::MAIN_SEPARATOR_STR;
use tempfile::TempDir;

mod common;
use common::{pctx, setup_test_project};

#[test]
fn test_files_list() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["files", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("lib.rs"))
        .stdout(predicate::str::contains("README.md"));
}

#[test]
fn test_files_list_quiet() {
    let dir = setup_test_project();

    let output = pctx()
        .current_dir(dir.path())
        .args(["files", "list", "--quiet"])
        .assert()
        .success();

    // Quiet mode should only output file paths, no counts or extra output
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Should have the file paths
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("lib.rs"));

    // Should NOT have the file count message (that goes to stderr in non-quiet mode)
    // Each line should be a path
    for line in stdout.lines() {
        assert!(!line.is_empty());
        assert!(!line.contains(" files")); // No "X files" count
    }
}

#[test]
fn test_files_tree() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["files", "tree"])
        .assert()
        .success()
        .stdout(predicate::str::contains("src"));
}

#[test]
fn test_json_files_list() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["--json", "files", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status": "success""#))
        .stdout(predicate::str::contains(r#""path":"#));
}

#[test]
fn test_config_init() {
    let dir = TempDir::new().unwrap();

    pctx()
        .current_dir(dir.path())
        .args(["config", "init"])
        .assert()
        .success();

    assert!(dir.path().join(".pctx.toml").exists());
}

#[test]
fn test_config_init_exists_error() {
    let dir = TempDir::new().unwrap();

    // Create config file first
    fs::write(dir.path().join(".pctx.toml"), "# existing").unwrap();

    pctx()
        .current_dir(dir.path())
        .args(["config", "init"])
        .assert()
        .code(5); // CONFLICT exit code
}

#[test]
fn test_config_init_force() {
    let dir = TempDir::new().unwrap();

    // Create config file first
    fs::write(dir.path().join(".pctx.toml"), "# existing").unwrap();

    pctx()
        .current_dir(dir.path())
        .args(["config", "init", "--force"])
        .assert()
        .success();

    let content = fs::read_to_string(dir.path().join(".pctx.toml")).unwrap();
    assert!(content.contains("pctx configuration"));
}

#[test]
fn test_config_defaults() {
    pctx()
        .args(["config", "defaults"])
        .assert()
        .success()
        .stdout(predicate::str::contains("node_modules"))
        .stdout(predicate::str::contains(".git"));
}

#[test]
fn test_config_show() {
    let dir = TempDir::new().unwrap();

    // Create a config file
    fs::write(
        dir.path().join(".pctx.toml"),
        r#"exclude = ["*.test.ts"]
max_lines = 1000
"#,
    )
    .unwrap();

    pctx()
        .current_dir(dir.path())
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("*.test.ts"));
}

#[test]
fn test_config_file_error_warning() {
    let dir = TempDir::new().unwrap();

    // Create an invalid config file
    fs::write(dir.path().join(".pctx.toml"), "invalid toml [[[").unwrap();
    fs::write(dir.path().join("test.txt"), "content").unwrap();

    // Should warn but continue
    pctx()
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Warning").and(predicate::str::contains("config")));
}

#[test]
fn test_completions_bash() {
    pctx()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn test_completions_zsh() {
    pctx()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef"));
}

#[test]
fn test_stdin_mode() {
    let dir = setup_test_project();

    // Use OS-appropriate paths in stdin
    let stdin_content = format!(
        "src{}main.rs\nsrc{}lib.rs\n",
        MAIN_SEPARATOR_STR, MAIN_SEPARATOR_STR
    );

    pctx()
        .current_dir(dir.path())
        .arg("--stdin")
        .write_stdin(stdin_content)
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("lib.rs"))
        .stdout(predicate::str::contains("README.md").not());
}

#[test]
fn test_stdin_empty() {
    let dir = TempDir::new().unwrap();

    pctx()
        .current_dir(dir.path())
        .arg("--stdin")
        .write_stdin("")
        .assert()
        .code(6) // NO_MATCH
        .stderr(predicate::str::contains("No files matched"));
}

#[test]
fn test_stdin_with_nonexistent_files() {
    let dir = setup_test_project();

    // Mix of existing and non-existing files
    let stdin_content = format!(
        "src{}main.rs\nnonexistent.txt\nsrc{}lib.rs\n",
        MAIN_SEPARATOR_STR, MAIN_SEPARATOR_STR
    );

    // Exit code 7 (PARTIAL) because nonexistent.txt is a file error
    pctx()
        .current_dir(dir.path())
        .args(["--stdin", "--verbose"])
        .write_stdin(stdin_content)
        .assert()
        .code(7)
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("lib.rs"));
}

#[test]
fn test_stdin_with_directories() {
    let dir = setup_test_project();

    // Include a directory - should expand it
    pctx()
        .current_dir(dir.path())
        .arg("--stdin")
        .write_stdin("src\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("lib.rs"));
}

#[test]
fn test_stdin_whitespace_lines_ignored() {
    let dir = setup_test_project();

    // Stdin with blank lines should be handled
    let stdin_content = format!("\n  \nsrc{}main.rs\n\n  \n", MAIN_SEPARATOR_STR);

    pctx()
        .current_dir(dir.path())
        .arg("--stdin")
        .write_stdin(stdin_content)
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn test_version() {
    pctx().arg("--version").assert().success();
}

#[test]
fn test_help() {
    pctx()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Generate LLM-ready context"))
        .stdout(predicate::str::contains("EXIT CODES"))
        .stdout(predicate::str::contains("RECURSION"));
}

#[test]
fn test_help_mentions_stdin() {
    pctx()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--stdin"))
        .stdout(predicate::str::contains("Read file paths from stdin"));
}

#[test]
fn test_help_mentions_no_truncation() {
    pctx()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--no-truncation"));
}

#[test]
fn test_verbose_flag() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .arg("--verbose")
        .assert()
        .success();
}

#[test]
fn test_quiet_flag() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .arg("--quiet")
        .assert()
        .success();
}
