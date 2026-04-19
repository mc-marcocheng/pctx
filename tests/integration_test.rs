//! Integration tests for pctx CLI

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::MAIN_SEPARATOR_STR;
use tempfile::TempDir;

fn pctx() -> Command {
    Command::cargo_bin("pctx").unwrap()
}

/// Setup a basic test project WITHOUT binary files
fn setup_test_project() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Create source files
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    fs::write(
        src_dir.join("main.rs"),
        r#"fn main() {
    println!("Hello, world!");
}
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("lib.rs"),
        r#"pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
"#,
    )
    .unwrap();

    // Create README
    fs::write(
        dir.path().join("README.md"),
        "# Test Project\n\nThis is a test.\n",
    )
    .unwrap();

    // Create Cargo.toml
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    )
    .unwrap();

    dir
}

/// Setup a test project WITH binary files for specific tests
fn setup_test_project_with_binary() -> TempDir {
    let dir = setup_test_project();
    // Create binary file (should be skipped)
    fs::write(dir.path().join("binary.bin"), [0x89, 0x50, 0x4E, 0x47]).unwrap();
    dir
}

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
fn test_exclude_pattern() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["--exclude", "*.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs").not())
        .stdout(predicate::str::contains("README.md"));
}

#[test]
fn test_include_pattern() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["--include", "*.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("README.md").not());
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
fn test_nonexistent_path() {
    pctx().arg("/nonexistent/path/12345").assert().code(3); // NOT_FOUND exit code
}

#[test]
fn test_binary_file_skipped() {
    let dir = setup_test_project_with_binary();

    // Binary file should be silently skipped (not in output, but no error)
    pctx()
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("binary.bin").not())
        .stdout(predicate::str::contains("main.rs"));
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
fn test_max_depth() {
    let dir = TempDir::new().unwrap();

    // Create nested structure
    // depth 1: a/
    // depth 2: a/file1.txt, a/b/
    // depth 3: a/b/file2.txt, a/b/c/
    // depth 4: a/b/c/file3.txt, a/b/c/d/
    // depth 5: a/b/c/d/file4.txt
    let deep_dir = dir.path().join("a").join("b").join("c").join("d");
    fs::create_dir_all(&deep_dir).unwrap();
    fs::write(dir.path().join("a").join("file1.txt"), "level 1").unwrap();
    fs::write(dir.path().join("a").join("b").join("file2.txt"), "level 2").unwrap();
    fs::write(
        dir.path().join("a").join("b").join("c").join("file3.txt"),
        "level 3",
    )
    .unwrap();
    fs::write(deep_dir.join("file4.txt"), "level 4").unwrap();

    // With max-depth 3, should see file1.txt and file2.txt but not file3.txt or file4.txt
    pctx()
        .current_dir(dir.path())
        .args(["--max-depth", "3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("file1.txt"))
        .stdout(predicate::str::contains("file2.txt"))
        .stdout(predicate::str::contains("file3.txt").not())
        .stdout(predicate::str::contains("file4.txt").not());
}

#[test]
fn test_truncation() {
    let dir = TempDir::new().unwrap();

    // Create a file with many lines
    let long_content: String = (1..=100).map(|i| format!("line {}\n", i)).collect();
    fs::write(dir.path().join("long.txt"), &long_content).unwrap();

    pctx()
        .current_dir(dir.path())
        .args([
            "--max-lines",
            "20",
            "--head-lines",
            "5",
            "--tail-lines",
            "5",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("line 1"))
        .stdout(predicate::str::contains("lines omitted"))
        .stdout(predicate::str::contains("line 100"));
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

#[test]
fn test_no_files_matched() {
    let dir = TempDir::new().unwrap();

    // Empty directory (or only excluded files)
    pctx()
        .current_dir(dir.path())
        .assert()
        .code(6) // NO_MATCH exit code
        .stderr(predicate::str::contains("No files matched"));
}

#[test]
fn test_no_files_matched_json() {
    let dir = TempDir::new().unwrap();

    pctx()
        .current_dir(dir.path())
        .arg("--json")
        .assert()
        .code(6)
        .stdout(predicate::str::contains(r#""status": "error""#))
        .stdout(predicate::str::contains("no_files_matched"));
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
fn test_include_hidden() {
    let dir = TempDir::new().unwrap();

    // Create a hidden file
    fs::write(dir.path().join(".hidden"), "hidden content").unwrap();
    fs::write(dir.path().join("visible.txt"), "visible content").unwrap();

    // Without --hidden, should not include hidden files
    pctx()
        .current_dir(dir.path())
        .args(["files", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".hidden").not())
        .stdout(predicate::str::contains("visible.txt"));

    // With --hidden, should include hidden files
    pctx()
        .current_dir(dir.path())
        .args(["files", "list", "--hidden"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".hidden"))
        .stdout(predicate::str::contains("visible.txt"));
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
fn test_multiple_paths() {
    let dir = TempDir::new().unwrap();

    // Create files in different directories
    let dir1 = dir.path().join("dir1");
    let dir2 = dir.path().join("dir2");
    fs::create_dir(&dir1).unwrap();
    fs::create_dir(&dir2).unwrap();

    fs::write(dir1.join("file1.txt"), "content 1").unwrap();
    fs::write(dir2.join("file2.txt"), "content 2").unwrap();

    pctx()
        .current_dir(dir.path())
        .args(["dir1", "dir2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("file1.txt"))
        .stdout(predicate::str::contains("file2.txt"));
}

#[test]
fn test_single_file_path() {
    let dir = TempDir::new().unwrap();

    fs::write(dir.path().join("single.txt"), "single file content").unwrap();
    fs::write(dir.path().join("other.txt"), "other content").unwrap();

    // Should only include the single file
    pctx()
        .current_dir(dir.path())
        .arg("single.txt")
        .assert()
        .success()
        .stdout(predicate::str::contains("single.txt"))
        .stdout(predicate::str::contains("other.txt").not());
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

// ==================== New tests ====================

#[test]
fn test_no_truncation_flag() {
    let dir = TempDir::new().unwrap();

    // Create a file with many lines
    let long_content: String = (1..=1000).map(|i| format!("line {}\n", i)).collect();
    fs::write(dir.path().join("long.txt"), &long_content).unwrap();

    // With --no-truncation, all lines should be present
    pctx()
        .current_dir(dir.path())
        .arg("--no-truncation")
        .assert()
        .success()
        .stdout(predicate::str::contains("line 1"))
        .stdout(predicate::str::contains("line 500"))
        .stdout(predicate::str::contains("line 1000"))
        .stdout(predicate::str::contains("lines omitted").not());
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
fn test_symlink_not_followed() {
    let dir = TempDir::new().unwrap();

    // Create a file and a symlink
    fs::write(dir.path().join("real.txt"), "real content").unwrap();

    // Create symlink (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(dir.path().join("real.txt"), dir.path().join("link.txt")).unwrap();

        // Both should appear, but symlink shouldn't cause issues
        pctx()
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("real.txt"));
    }
}

#[test]
fn test_unicode_filenames() {
    let dir = TempDir::new().unwrap();

    // Create files with unicode names
    fs::write(dir.path().join("文件.txt"), "Chinese filename").unwrap();
    fs::write(dir.path().join("файл.txt"), "Russian filename").unwrap();
    fs::write(dir.path().join("αρχείο.txt"), "Greek filename").unwrap();

    pctx()
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Chinese filename"))
        .stdout(predicate::str::contains("Russian filename"))
        .stdout(predicate::str::contains("Greek filename"));
}

#[test]
fn test_very_long_line() {
    let dir = TempDir::new().unwrap();

    // Create a file with a very long line
    let long_line = "x".repeat(10000);
    fs::write(dir.path().join("long_line.txt"), &long_line).unwrap();

    // Default truncation should handle it
    let output = pctx().current_dir(dir.path()).assert().success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("chars omitted"));
}

#[test]
fn test_pattern_double_star() {
    let dir = TempDir::new().unwrap();

    // Create nested test files
    let test_dir = dir.path().join("src").join("tests").join("unit");
    fs::create_dir_all(&test_dir).unwrap();
    fs::write(test_dir.join("test_foo.rs"), "test").unwrap();
    fs::write(dir.path().join("src").join("main.rs"), "main").unwrap();

    // Exclude with **
    pctx()
        .current_dir(dir.path())
        .args(["--exclude", "**/tests/**"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("test_foo.rs").not());
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
fn test_empty_file_handling() {
    let dir = TempDir::new().unwrap();

    // Create an empty file
    fs::write(dir.path().join("empty.txt"), "").unwrap();

    pctx()
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("empty.txt"));
}

#[test]
fn test_file_with_only_whitespace() {
    let dir = TempDir::new().unwrap();

    fs::write(dir.path().join("whitespace.txt"), "   \n\n   \t\n").unwrap();

    pctx()
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("whitespace.txt"));
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
fn test_conflicting_truncation_flags() {
    let dir = setup_test_project();

    // --no-truncation conflicts with --max-lines
    pctx()
        .current_dir(dir.path())
        .args(["--no-truncation", "--max-lines", "100"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_large_file_skipped() {
    let dir = TempDir::new().unwrap();

    // Create a file larger than 1KB
    let content = "x".repeat(2000);
    fs::write(dir.path().join("large.txt"), &content).unwrap();
    fs::write(dir.path().join("small.txt"), "small").unwrap();

    // With very small max-size, large file should be excluded
    pctx()
        .current_dir(dir.path())
        .args(["--max-size", "1"]) // 1 KB
        .assert()
        .success()
        .stdout(predicate::str::contains("small.txt"))
        .stdout(predicate::str::contains("large.txt").not());
}

#[test]
fn test_negation_pattern_warning() {
    let dir = setup_test_project();

    // Negation patterns should produce a warning, but still succeed
    pctx()
        .current_dir(dir.path())
        .args(["--exclude", "!*.rs"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "negation patterns are not supported",
        ));
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
fn test_absolute_paths_flag() {
    let dir = setup_test_project();
    let dir_path = dir.path().to_string_lossy().to_string();

    let output = pctx()
        .current_dir(dir.path())
        .arg("--absolute-paths")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Should contain absolute path
    assert!(stdout.contains(&dir_path));
}

#[test]
fn test_no_default_excludes() {
    let dir = TempDir::new().unwrap();

    // Create a regular file outside node_modules
    fs::write(dir.path().join("regular.txt"), "regular content").unwrap();

    // Create a node_modules directory with a file
    let node_modules = dir.path().join("node_modules");
    fs::create_dir(&node_modules).unwrap();
    fs::write(node_modules.join("package.json"), r#"{"name": "test"}"#).unwrap();

    // By default, node_modules should be excluded
    pctx()
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("regular.txt"))
        .stdout(predicate::str::contains("node_modules").not());

    // With --no-default-excludes, it should be included
    pctx()
        .current_dir(dir.path())
        .args(["--no-default-excludes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("node_modules"));
}

#[test]
fn test_max_size_filter() {
    let dir = TempDir::new().unwrap();

    // Create files of different sizes
    fs::write(dir.path().join("small.txt"), "small").unwrap();
    fs::write(dir.path().join("medium.txt"), "x".repeat(1500)).unwrap();

    // With max-size 1KB, only small.txt should be included
    pctx()
        .current_dir(dir.path())
        .args(["--max-size", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("small.txt"))
        .stdout(predicate::str::contains("medium.txt").not());
}
