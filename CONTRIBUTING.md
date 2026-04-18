# Contributing to pctx

Thank you for your interest in contributing to pctx! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Style Guidelines](#style-guidelines)
- [Reporting Issues](#reporting-issues)

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Please be respectful and constructive in all interactions.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Set up the development environment
4. Create a branch for your changes
5. Make your changes with tests
6. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.70 or later (install via [rustup](https://rustup.rs/))
- Git

### Building

```bash
# Clone your fork
git clone https://github.com/mc-marcocheng/pctx
cd pctx

# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run directly
cargo run -- --help
```

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run integration tests only
cargo test --test integration_test

# Run unit tests only
cargo test --lib
```

### Other Commands

```bash
# Check code without building
cargo check

# Run clippy lints
cargo clippy -- -D warnings

# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Generate documentation
cargo doc --open

# Run benchmarks (if any)
cargo bench
```

## Project Structure

```
pctx/
├── src/
│   ├── main.rs           # Entry point, CLI handling
│   ├── lib.rs            # Library exports
│   ├── cli.rs            # Command-line argument definitions
│   ├── error.rs          # Error types and handling
│   ├── stats.rs          # Statistics collection
│   ├── exit_codes.rs     # Exit code constants
│   ├── config/
│   │   ├── mod.rs        # Configuration merging
│   │   ├── file.rs       # Config file loading
│   │   └── defaults.rs   # Default exclusion patterns
│   ├── content/
│   │   ├── mod.rs        # Content processing coordination
│   │   ├── reader.rs     # File reading with encoding handling
│   │   └── truncator.rs  # Line/content truncation
│   ├── filter/
│   │   ├── mod.rs        # Filter coordination
│   │   ├── binary.rs     # Binary file detection
│   │   └── patterns.rs   # Gitignore-style pattern matching
│   ├── output/
│   │   ├── mod.rs        # Output module exports
│   │   ├── formatter.rs  # Format output (markdown/xml/plain)
│   │   ├── tree.rs       # File tree generation
│   │   ├── clipboard.rs  # Clipboard operations
│   │   ├── file.rs       # File output
│   │   └── json_types.rs # JSON response structures
│   └── scanner/
│       ├── mod.rs        # File discovery coordination
│       ├── walker.rs     # Directory walking
│       └── git.rs        # Git-aware scanning
├── tests/
│   └── integration_test.rs  # Integration tests
├── Cargo.toml
├── README.md
├── CONTRIBUTING.md
└── LICENSE
```

### Module Responsibilities

| Module | Responsibility |
|--------|---------------|
| `cli` | Define CLI arguments and commands using clap |
| `config` | Load and merge configuration from files and arguments |
| `content` | Read files, handle encoding, truncate content |
| `filter` | Detect binary files, match patterns |
| `output` | Format and write output to various destinations |
| `scanner` | Discover files respecting gitignore and filters |
| `error` | Define error types with codes and suggestions |
| `stats` | Collect and report statistics |

## Making Changes

### Branch Naming

Use descriptive branch names:

- `feature/add-stdin-support`
- `fix/binary-detection-edge-case`
- `docs/improve-readme`
- `refactor/simplify-pattern-matching`

### Commit Messages

Follow conventional commit style:

```
type(scope): short description

Longer description if needed.

Fixes #123
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Examples:
```
feat(cli): add --stdin flag for reading file paths from stdin

fix(binary): improve detection for files without extensions

docs(readme): add examples for CI/CD integration

test(integration): add tests for unicode filenames
```

### Adding New Features

1. **Discuss first**: For significant changes, open an issue to discuss the approach
2. **Update CLI**: Add arguments in `src/cli.rs`
3. **Update config**: If configurable, update `src/config/`
4. **Implement**: Add implementation in appropriate module
5. **Add tests**: Both unit tests and integration tests
6. **Update docs**: Update README.md and help text
7. **Update CHANGELOG**: Document the change

### Adding New Output Formats

1. Add variant to `ContentFormat` enum in `src/cli.rs`
2. Implement formatting in `src/output/formatter.rs`
3. Add tests for the new format
4. Update README with examples

### Adding New Filter Types

1. Add filter logic in `src/filter/`
2. Add CLI arguments if user-configurable
3. Integrate with `Scanner` in `src/scanner/mod.rs`
4. Add tests

## Testing

### Test Requirements

- All new features must have tests
- All bug fixes should have regression tests
- Tests must pass on CI before merging

### Types of Tests

**Unit Tests** (in source files):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specific_function() {
        // Test implementation
    }
}
```

**Integration Tests** (in `tests/`):
```rust
#[test]
fn test_end_to_end_feature() {
    let dir = setup_test_project();

    pctx()
        .current_dir(dir.path())
        .args(["--flag"])
        .assert()
        .success()
        .stdout(predicate::str::contains("expected"));
}
```

### Test Helpers

The integration tests use these helpers:

```rust
// Create a Command for pctx
fn pctx() -> Command {
    Command::cargo_bin("pctx").unwrap()
}

// Set up a test project with common files
fn setup_test_project() -> TempDir {
    // Creates src/main.rs, src/lib.rs, README.md, etc.
}
```

### What to Test

- Happy path (normal usage)
- Edge cases (empty files, unicode, very long lines)
- Error cases (missing files, permission errors)
- Flag combinations and conflicts
- JSON output structure
- Exit codes

## Pull Request Process

1. **Ensure tests pass**: `cargo test`
2. **Ensure no warnings**: `cargo clippy -- -D warnings`
3. **Format code**: `cargo fmt`
4. **Update documentation** if needed
5. **Write clear PR description**:
   - What changes were made
   - Why the changes were made
   - How to test the changes
   - Related issues

### PR Checklist

```markdown
- [ ] Tests pass locally (`cargo test`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] Documentation updated (if applicable)
- [ ] CHANGELOG updated (if applicable)
- [ ] Commits are clean and well-described
```

### Review Process

1. Maintainers will review within a few days
2. Address feedback with new commits or amendments
3. Once approved, maintainer will merge
4. Delete your branch after merge

## Style Guidelines

### Rust Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` defaults
- Use `clippy` and fix all warnings
- Prefer explicit error handling over `.unwrap()` in library code
- Use `.expect("reason")` only when failure indicates a bug

### Code Organization

```rust
// Order of items in a module:
// 1. Module-level documentation
// 2. Imports (std, external crates, internal)
// 3. Constants
// 4. Type definitions
// 5. Implementations
// 6. Functions
// 7. Tests

//! Module documentation

use std::path::Path;

use serde::Serialize;

use crate::error::PctxError;

const MAX_SIZE: usize = 1024;

pub struct MyType {
    // fields
}

impl MyType {
    pub fn new() -> Self {
        // ...
    }
}

pub fn public_function() {
    // ...
}

fn private_function() {
    // ...
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example() {
        // ...
    }
}
```

### Error Handling

```rust
// Good: Return Result with descriptive error
pub fn read_file(path: &Path) -> Result<String, PctxError> {
    std::fs::read_to_string(path)
        .map_err(|e| PctxError::Io(e))
}

// Good: Add context to errors
pub fn process_config(path: &Path) -> Result<Config, PctxError> {
    let content = std::fs::read_to_string(path)
        .map_err(PctxError::Io)?;

    toml::from_str(&content)
        .map_err(PctxError::Toml)
}

// Avoid: Using .unwrap() in library code
// let content = std::fs::read_to_string(path).unwrap();
```

### Documentation

```rust
/// Brief description of the function.
///
/// More detailed explanation if needed.
///
/// # Arguments
///
/// * `path` - The file path to process
/// * `config` - Configuration options
///
/// # Returns
///
/// Returns the processed content or an error if the file cannot be read.
///
/// # Errors
///
/// Returns `PctxError::FileNotFound` if the path doesn't exist.
/// Returns `PctxError::PermissionDenied` if the file isn't readable.
///
/// # Examples
///
/// ```
/// use pctx::process_file;
///
/// let content = process_file("src/main.rs", &config)?;
/// ```
pub fn process_file(path: &str, config: &Config) -> Result<String, PctxError> {
    // ...
}
```

### CLI Arguments

When adding CLI arguments:

```rust
/// Clear, concise help text for the flag
#[arg(
    short,           // Single-letter shorthand if appropriate
    long,            // Full name
    value_name = "NAME",  // Placeholder in help
    default_value = "default",
    help = "Short help shown in --help",
    long_help = "Longer help with examples\n\n\
                 Example: pctx --flag value"
)]
pub flag_name: Type,
```

## Reporting Issues

### Bug Reports

Include:

1. **pctx version**: `pctx --version`
2. **OS and version**: e.g., "macOS 14.0" or "Ubuntu 22.04"
3. **Steps to reproduce**: Minimal commands to trigger the bug
4. **Expected behavior**: What you expected to happen
5. **Actual behavior**: What actually happened
6. **Error output**: Full error message if any

Template:
````markdown
**Version**: pctx 0.1.0
**OS**: macOS 14.0

**Steps to reproduce**:
1. Create a file with: `echo "test" > test.txt`
2. Run: `pctx --flag`

**Expected**: Should output formatted content
**Actual**: Crashes with error: "..."

**Full output**:
```
<paste full terminal output>
```
````

### Feature Requests

Include:

1. **Use case**: Why do you need this feature?
2. **Proposed solution**: How do you envision it working?
3. **Alternatives considered**: Other approaches you've thought of
4. **Examples**: Command-line examples of proposed usage

Template:
````markdown
**Use case**: I want to process only files changed in a git commit

**Proposed solution**: Add a `--git-diff` flag that accepts a commit range

**Example usage**:
```bash
pctx --git-diff HEAD~5..HEAD
pctx --git-diff origin/main
```

**Alternatives considered**:
- Using `git diff --name-only | pctx --stdin` (works but verbose)
````

Thank you for contributing! 🎉
