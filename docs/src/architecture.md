# Architecture & Developer Guide

`pctx` is implemented in Rust as both a CLI application and a library crate.

The CLI entry point lives in `src/main.rs`. Most reusable functionality is exposed through modules under `src/lib.rs`.

## High-level pipeline

A normal `pctx` run follows this pipeline:

```text
CLI args
   │
   ▼
Config resolution
   │
   ▼
File discovery
   │
   ▼
Filtering
   │
   ▼
Content reading
   │
   ▼
Truncation
   │
   ▼
Formatting
   │
   ▼
Destination output
```

In JSON mode, the same pipeline is used, but responses are wrapped in structured JSON.

## Module overview

| Module | Purpose |
|--------|---------|
| `cli` | Defines command-line arguments and subcommands with `clap` |
| `config` | Resolves defaults, `.pctx.toml`, and CLI overrides |
| `scanner` | Discovers candidate files from paths, git, stdin, or directory walking |
| `filter` | Handles binary detection and gitignore-style include/exclude patterns |
| `content` | Reads files and builds `FileEntry` values |
| `content::truncator` | Applies file and long-line truncation |
| `output` | Formats content and writes to stdout, files, or clipboard |
| `output::json_types` | Defines the structured JSON API |
| `output::tree` | Builds and renders file trees |
| `stats` | Tracks file counts, sizes, truncation counts, and token estimates |
| `error` | Defines typed errors, suggestions, and error codes |
| `exit_codes` | Defines stable process exit codes |

## CLI layer

`src/cli.rs` defines:

- global options such as `--json`, `--verbose`, `--quiet`, and `--no-color`
- generate options, used when no subcommand is supplied
- `files list`
- `files tree`
- `config show`
- `config init`
- `config defaults`
- `completions`

The CLI is intentionally designed for both humans and automation:

- Human-readable output goes to stdout or stderr depending on purpose.
- JSON result payloads go to stdout.
- Diagnostic messages go to stderr.
- Exit codes communicate command outcome.

## Configuration

Configuration is represented by `Config` in `src/config/mod.rs`.

Sources are merged in this order:

1. CLI arguments
2. `.pctx.toml`
3. built-in defaults

The config file type is `FileConfig` in `src/config/file.rs`.

Currently, `.pctx.toml` supports:

- `exclude`
- `include`
- truncation settings

Built-in exclusions are defined in `src/config/defaults.rs`.

## Scanning

File discovery is handled by `Scanner` in `src/scanner/mod.rs`.

There are two primary scanning paths:

1. **Configured paths**
   - Used by normal `pctx` generation.
   - Defaults to `.`.
   - Accepts files and directories.

2. **Explicit stdin paths**
   - Used with `--stdin`.
   - Reads one path per line.
   - Expands directories recursively.

Directory traversal is implemented in `src/scanner/walker.rs` using the `ignore` crate.

When gitignore support is enabled and the target is inside a git repository, `src/scanner/git.rs` can use:

```bash
git ls-files -z --cached --others --exclude-standard
```

This gives git-aware file discovery for tracked and untracked files while respecting standard git exclusions.

## Filtering model

Filtering happens in several layers:

1. **Traversal-level filtering**
   - Hidden paths may be skipped during walking.
   - Gitignore rules may be applied during walking or git scanning.
   - Maximum depth may limit recursion.

2. **Pattern filtering**
   - Built-in excludes
   - Config-file excludes
   - CLI excludes
   - Config-file includes
   - CLI includes

3. **File validation**
   - Maximum file size
   - Binary detection

Hidden-path filtering is independent from default exclusions and gitignore rules.

## Pattern matching

`src/filter/patterns.rs` implements gitignore-style matching using the `glob` crate.

Important behavior:

- Simple patterns can match path components anywhere.
- Multi-component patterns can match nested paths.
- Trailing slash patterns match directories.
- Leading `/` anchors a pattern to the scan root.
- Negation patterns are not supported.

## Binary detection

`src/filter/binary.rs` detects binary files using:

- known binary extensions
- common magic-byte signatures
- null-byte checks
- non-printable byte ratio

Binary files are skipped before content processing.

## Content processing

`ContentProcessor` in `src/content/mod.rs` turns paths into `FileEntry` values.

A `FileEntry` contains:

- absolute path
- relative display path
- extension
- original byte count
- original line count
- processed line count
- truncation metadata
- processed content

Files are read by `src/content/reader.rs`.

Invalid UTF-8 is converted lossily rather than failing immediately, which makes pctx tolerant of mixed or imperfect text encodings.

## Truncation

`src/content/truncator.rs` handles two truncation modes:

1. **File-level truncation**
   - Triggered when a file exceeds `max_lines`.
   - Preserves the configured head and tail line counts.
   - Inserts an omission marker.

2. **Line-level truncation**
   - Triggered when a line exceeds `max_line_length`.
   - Preserves the configured head and tail character counts.
   - Inserts a character omission marker.

`0` disables the corresponding truncation limit.

## Formatting

Formatting lives in `src/output/formatter.rs`.

Supported formats:

| Format | Description |
|--------|-------------|
| `markdown` | File labels and fenced code blocks |
| `xml` | XML document with file contents in CDATA |
| `plain` | Simple text separators |

Markdown formatting automatically grows code fences if file content already contains triple backticks.

XML formatting escapes attributes and protects against CDATA termination sequences.

## Output destinations

Output destination handling is split across:

- `src/output/stdout.rs`
- `src/output/file.rs`
- `src/output/clipboard.rs`

File output uses an atomic create-new behavior by default, so existing files are not overwritten unless `--force` is supplied.

## JSON API

Structured JSON types are defined in `src/output/json_types.rs`.

Top-level responses are:

- `success`
- `partial`
- `error`

Errors include:

- machine-readable code
- message
- optional input context
- optional suggestion
- transient flag
- exit code

The JSON API is intended for scripts, CI jobs, and agent harnesses.

## Errors and exit codes

`src/error.rs` defines `PctxError`.

Each error can provide:

- display message
- machine-readable code
- suggested fix
- transient/non-transient classification
- structured input context
- exit code

Stable exit code constants are defined in `src/exit_codes.rs`.

When changing exit codes, update:

1. `src/exit_codes.rs`
2. CLI help text in `src/cli.rs`
3. user documentation
4. scripts or integrations that depend on the code

## Statistics

`src/stats.rs` tracks:

- file count
- total original lines
- total original bytes
- truncated file count
- skipped count
- approximate token estimate
- duration

Token estimation uses a tokenizer when compiled with token support; otherwise it falls back to an approximate character-based estimate.

## Tests

The project includes unit tests for:

- config loading and merging
- default exclusions
- content processing
- truncation
- binary detection
- pattern matching
- output formatting
- tree rendering
- stats formatting
- error metadata

Snapshot tests are used for formatter and tree output.

Run tests with:

```bash
cargo test
```

If an intentional formatting change affects snapshots, review/update snapshots with the normal `insta` workflow.

## Developer workflow

Useful commands:

```bash
# Format code
cargo fmt

# Lint
cargo clippy --all-targets --all-features

# Run tests
cargo test

# Build release binary
cargo build --release

# Try the CLI locally
cargo run -- --help
cargo run -- --dry-run --tree
cargo run -- files list --quiet
```

Build and serve documentation:

```bash
cd docs
mdbook serve
```

## Extension points

Common areas to extend:

| Goal | Likely files |
|------|--------------|
| Add a CLI flag | `src/cli.rs`, `src/config/mod.rs`, docs |
| Add a config key | `src/config/file.rs`, `src/config/mod.rs`, docs |
| Add an output format | `src/cli.rs`, `src/output/formatter.rs`, snapshot tests |
| Change default excludes | `src/config/defaults.rs`, docs |
| Improve pattern behavior | `src/filter/patterns.rs` |
| Change JSON response shape | `src/output/json_types.rs`, docs |
| Add a new subcommand | `src/cli.rs`, `src/main.rs`, docs |

## Notes for maintainers

- Keep CLI help and documentation in sync.
- Keep JSON response shapes stable where possible.
- Treat exit codes as public API.
- Prefer adding tests for filtering and formatting changes.
- Be careful when changing hidden-file behavior; it is intentionally independent from default exclusions and gitignore rules.
