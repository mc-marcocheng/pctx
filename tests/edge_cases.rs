//! Edge cases: unusual content, filesystem quirks, and error paths.

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod common;
use common::{pctx, setup_test_project_with_binary};

#[test]
fn test_nonexistent_path() {
    pctx().arg("/nonexistent/path/12345").assert().code(3); // NOT_FOUND exit code
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
