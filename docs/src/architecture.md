# Architecture & Developer Guide

`pctx` is built in Rust with a modular design aimed at fast file discovery and robust content formatting. It provides both a command-line interface and a library crate that you can use programmatically.

## Core Modules

The library (`src/lib.rs`) is divided into several focused modules:

### 1. `cli`
Defines the command-line interface using the `clap` crate. It handles parsing commands, options (like `--max-depth`, `--json`), and truncation thresholds. The CLI is designed to provide structured output (`--json` mode) for programmatic consumers and standard human-readable streams.

### 2. `config`
Manages configuration resolution. Settings are prioritized in the following order:
1. Command-line arguments.
2. Configuration file (`.pctx.toml`).
3. Built-in defaults.

### 3. `scanner`
Responsible for file discovery and validation.
- Uses robust tools like `walkdir` to traverse directories recursively.
- Respects depth limits.
- Validates that paths exist and are accessible.

### 4. `filter`
The filtering engine handles `.gitignore` logic and custom gitignore-style glob patterns.
- Applies standard built-in exclusions (e.g., `node_modules`, `.git`, `target`).
- Excludes binary files or overly large files.
- Evaluates `--include` and `--exclude` rules dynamically.

### 5. `content` & `truncation`
Once files are discovered and filtered, this module processes the raw text.
- Reads file contents efficiently.
- Applies line and character truncation thresholds to ensure LLM prompt windows aren't overwhelmed.
- Automatically preserves the "head" and "tail" of files or long lines to provide maximum semantic context.

### 6. `output`
Handles the final formatting and writing.
- Formats context into Markdown, XML, or Plain text.
- Outputs via `stdout`, clipboard (`arboard`), or file output.
- Generates JSON structures for the `--json` option.
- Optionally produces a filesystem tree via the `tree` module.

### 7. `stats`
Calculates usage statistics (e.g., total files, token estimation, total bytes) to help users gauge the size of the generated context.
