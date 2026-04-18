//! Integration tests for pctx CLI

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn pctx() -> Command {
    Command::cargo_bin("pctx").unwrap()
}

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

    // Create binary file (should be skipped)
    fs::write(dir.path().join("binary.bin"), &[0x89, 0x50, 0x4E, 0x47]).unwrap();

    dir
}

#[test]
fn test_basic_output() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("`src/main.rs`:"))
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
        .stdout(predicate::str::contains(r#""path": "src/main.rs""#));
}

#[test]
fn test_files_list() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["files", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("src/lib.rs"))
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
    assert!(stdout.contains("src/main.rs"));
    assert!(stdout.contains("src/lib.rs"));

    // Should NOT have the file count message (that goes to stderr in non-quiet mode)
    // Each line should be a path
    for line in stdout.lines() {
        assert!(!line.is_empty());
        assert!(!line.contains("files")); // No "X files" count
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
        .stderr(predicate::str::contains("src"));
}

#[test]
fn test_exclude_pattern() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["--exclude", "*.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("`src/main.rs`:").not())
        .stdout(predicate::str::contains("`README.md`:"));
}

#[test]
fn test_include_pattern() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["--include", "*.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("`src/main.rs`:"))
        .stdout(predicate::str::contains("`README.md`:").not());
}

#[test]
fn test_output_to_file() {
    let dir = setup_test_project();
    let output_file = dir.path().join("output.md");

    pctx()
        .current_dir(dir.path())
        .args(["--output", "output.md"])
        .assert()
        .success();

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("src/main.rs"));
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
        .args(["--output", "output.md", "--force"])
        .assert()
        .success();

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("src/main.rs"));
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
        .stdout(predicate::str::contains("=== src/main.rs ==="));
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
        .stderr(predicate::str::contains("src/main.rs"));
}

#[test]
fn test_dry_run_json() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["--dry-run", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status": "success""#))
        .stdout(predicate::str::contains(r#""path": "src/main.rs""#));
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
    pctx()
        .arg("/nonexistent/path/12345")
        .assert()
        .code(3); // NOT_FOUND exit code
}

#[test]
fn test_binary_file_skipped() {
    let dir = setup_test_project();

    // Output shouldn't contain the binary file
    pctx()
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("binary.bin").not());
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
    let deep_dir = dir.path().join("a/b/c/d");
    fs::create_dir_all(&deep_dir).unwrap();
    fs::write(dir.path().join("a/file1.txt"), "level 1").unwrap();
    fs::write(dir.path().join("a/b/file2.txt"), "level 2").unwrap();
    fs::write(dir.path().join("a/b/c/file3.txt"), "level 3").unwrap();
    fs::write(dir.path().join("a/b/c/d/file4.txt"), "level 4").unwrap();

    // With max-depth 2, should only see files up to 2 levels deep
    pctx()
        .current_dir(dir.path())
        .args(["--max-depth", "2"])
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
        .args(["--max-lines", "20", "--head-lines", "5", "--tail-lines", "5"])
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

    pctx()
        .current_dir(dir.path())
        .args(["--format", "xml"])
        .assert()
        .success()
        // The ]]> should be escaped
        .stdout(predicate::str::contains("]]>").not().or(
            // Or it should be properly escaped in CDATA
            predicate::str::contains("]]]]><![CDATA[>"),
        ));
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
    let content: String = (1..=50).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
    fs::write(dir.path().join("test.txt"), &content).unwrap();

    let output = pctx()
        .current_dir(dir.path())
        .args(["--max-lines", "10", "--head-lines", "3", "--tail-lines", "2"])
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