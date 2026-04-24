//! Shared helpers for pctx integration tests.

#![allow(dead_code)]

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

pub fn pctx() -> Command {
    Command::cargo_bin("pctx").unwrap()
}

/// Setup a basic test project WITHOUT binary files
pub fn setup_test_project() -> TempDir {
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
pub fn setup_test_project_with_binary() -> TempDir {
    let dir = setup_test_project();
    // Create binary file (should be skipped)
    fs::write(dir.path().join("binary.bin"), [0x89, 0x50, 0x4E, 0x47]).unwrap();
    dir
}
