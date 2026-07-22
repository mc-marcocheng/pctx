//! Integration tests for the CLI-compatibility feature set:
//! --config/--no-config, --stdin0/--paths-file0, --path-alias, `capabilities`,
//! and JSON partial-result reporting.

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod common;
use common::{pctx, setup_test_project};

#[test]
fn test_config_flag_loads_specified_file() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("custom.toml");
    fs::write(&config_path, r#"exclude = ["*.custom-exclude"]"#).unwrap();

    pctx()
        .current_dir(dir.path())
        .args(["--config", config_path.to_str().unwrap(), "config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("custom-exclude"));
}

#[test]
fn test_config_flag_missing_file_exit_code_3() {
    let dir = TempDir::new().unwrap();

    pctx()
        .current_dir(dir.path())
        .args(["--config", "does-not-exist.toml"])
        .assert()
        .code(3);
}

#[test]
fn test_config_flag_malformed_file_exit_code_2() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("bad.toml");
    fs::write(&config_path, "this is = not [valid toml").unwrap();

    pctx()
        .current_dir(dir.path())
        .args(["--config", config_path.to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn test_no_config_prevents_parent_discovery() {
    let dir = setup_test_project();
    fs::write(dir.path().join(".pctx.toml"), r#"exclude = ["main.rs"]"#).unwrap();

    // Without --no-config, main.rs is excluded by the discovered config.
    pctx()
        .current_dir(dir.path())
        .args(["files", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs").not());

    // With --no-config, the exclude rule is ignored.
    pctx()
        .current_dir(dir.path())
        .args(["--no-config", "files", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn test_config_conflicts_with_no_config() {
    pctx()
        .args(["--config", "x.toml", "--no-config"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_stdin_variants_conflict() {
    pctx().args(["--stdin", "--stdin0"]).assert().failure();

    pctx()
        .args(["--stdin", "--paths-file0", "foo"])
        .assert()
        .failure();

    pctx()
        .args(["--stdin0", "--paths-file0", "foo"])
        .assert()
        .failure();
}

#[test]
fn test_stdin0_reads_nul_delimited_paths_with_spaces() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file with spaces.txt"), "content").unwrap();
    fs::write(dir.path().join("other.txt"), "content2").unwrap();

    let stdin_content = format!(
        "{}\0{}\0",
        dir.path().join("file with spaces.txt").display(),
        dir.path().join("other.txt").display()
    );

    pctx()
        .current_dir(dir.path())
        .arg("--stdin0")
        .write_stdin(stdin_content)
        .assert()
        .success()
        .stdout(predicate::str::contains("file with spaces.txt"))
        .stdout(predicate::str::contains("other.txt"));
}

#[test]
fn test_stdin0_empty_input_no_match() {
    let dir = TempDir::new().unwrap();

    pctx()
        .current_dir(dir.path())
        .arg("--stdin0")
        .write_stdin("")
        .assert()
        .code(6);
}

#[test]
fn test_paths_file0_reads_nul_delimited_paths() {
    let dir = setup_test_project();
    let paths_file = dir.path().join("paths.bin");

    let content = format!(
        "{}\0{}\0",
        dir.path().join("src").join("main.rs").display(),
        dir.path().join("src").join("lib.rs").display()
    );
    fs::write(&paths_file, content).unwrap();

    pctx()
        .current_dir(dir.path())
        .args(["--paths-file0", paths_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("lib.rs"))
        .stdout(predicate::str::contains("README.md").not());
}

#[test]
fn test_paths_file0_empty_input_no_match() {
    let dir = TempDir::new().unwrap();
    let paths_file = dir.path().join("empty.bin");
    fs::write(&paths_file, "").unwrap();

    pctx()
        .current_dir(dir.path())
        .args(["--paths-file0", paths_file.to_str().unwrap()])
        .assert()
        .code(6);
}

#[test]
fn test_path_alias_unrelated_roots() {
    let dir_a = TempDir::new().unwrap();
    let dir_b = TempDir::new().unwrap();
    fs::write(dir_a.path().join("main.rs"), "fn main() {}").unwrap();
    fs::write(dir_b.path().join("lib.rs"), "pub fn f() {}").unwrap();

    let alias_a = format!("api={}", dir_a.path().display());
    let alias_b = format!("shared={}", dir_b.path().display());

    pctx()
        .args([
            dir_a.path().join("main.rs").to_str().unwrap(),
            dir_b.path().join("lib.rs").to_str().unwrap(),
            "--path-alias",
            &alias_a,
            "--path-alias",
            &alias_b,
            "--format",
            "plain",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("api/main.rs").or(predicate::str::contains("api\\main.rs")),
        )
        .stdout(
            predicate::str::contains("shared/lib.rs")
                .or(predicate::str::contains("shared\\lib.rs")),
        );
}

#[test]
fn test_path_alias_nested_uses_most_specific() {
    let dir = TempDir::new().unwrap();
    let nested = dir.path().join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("file.rs"), "content").unwrap();

    let outer_alias = format!("outer={}", dir.path().display());
    let inner_alias = format!("inner={}", nested.display());

    pctx()
        .args([
            nested.join("file.rs").to_str().unwrap(),
            "--path-alias",
            &outer_alias,
            "--path-alias",
            &inner_alias,
            "--format",
            "plain",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("inner/file.rs")
                .or(predicate::str::contains("inner\\file.rs")),
        )
        .stdout(predicate::str::contains("outer/nested").not());
}

#[test]
fn test_path_alias_duplicate_fails() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file.rs"), "content").unwrap();
    let alias = format!("dup={}", dir.path().display());

    pctx()
        .args([
            dir.path().join("file.rs").to_str().unwrap(),
            "--path-alias",
            &alias,
            "--path-alias",
            &alias,
        ])
        .assert()
        .code(2);
}

#[test]
fn test_path_alias_invalid_format_fails() {
    pctx()
        .args(["--path-alias", "not-a-valid-alias"])
        .assert()
        .code(2);
}

#[test]
fn test_path_alias_json_metadata_uses_alias() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    let alias = format!("api={}", dir.path().display());

    pctx()
        .args([
            "--json",
            dir.path().join("main.rs").to_str().unwrap(),
            "--path-alias",
            &alias,
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("api/main.rs").or(predicate::str::contains("api\\\\main.rs")),
        );
}

#[test]
fn test_absolute_paths_overrides_alias() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    let alias = format!("api={}", dir.path().display());

    pctx()
        .args([
            dir.path().join("main.rs").to_str().unwrap(),
            "--path-alias",
            &alias,
            "--absolute-paths",
            "--format",
            "plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("api/main.rs").not())
        .stdout(predicate::str::contains("api\\main.rs").not());
}

#[test]
fn test_capabilities_json() {
    pctx()
        .args(["--json", "capabilities"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""stdin0": true"#))
        .stdout(predicate::str::contains(r#""paths_file0": true"#))
        .stdout(predicate::str::contains(r#""path_aliases": true"#));
}

#[test]
fn test_capabilities_human_readable() {
    pctx()
        .arg("capabilities")
        .assert()
        .success()
        .stdout(predicate::str::contains("stdin0: true"))
        .stdout(predicate::str::contains("paths_file0: true"));
}

#[test]
fn test_generate_json_partial_on_scan_errors() {
    // Exercises the same JsonResponse::Partial/FileError plumbing that
    // `files list --json` and `files tree --json` now share via
    // scan_errors_to_file_errors/scan_exit_code.
    let dir = setup_test_project();
    let paths_file = dir.path().join("paths.bin");
    let content = format!(
        "{}\0{}\0",
        dir.path().join("src").join("main.rs").display(),
        dir.path().join("nonexistent.txt").display()
    );
    fs::write(&paths_file, content).unwrap();

    pctx()
        .current_dir(dir.path())
        .args(["--json", "--paths-file0", paths_file.to_str().unwrap()])
        .assert()
        .code(7)
        .stdout(predicate::str::contains(r#""status": "partial""#));
}
