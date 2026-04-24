//! Include/exclude patterns, size/depth/truncation filters, and path selection.

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod common;
use common::{pctx, setup_test_project};

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
