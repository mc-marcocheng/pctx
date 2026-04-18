# pctx

Generate LLM-ready context from your codebase.

`pctx` scans your project, respects `.gitignore`, filters files intelligently, and outputs formatted context ready to paste into ChatGPT, Claude, or any other LLM.

## Features

- 🚀 **Fast** - Built in Rust, handles large codebases efficiently
- 🎯 **Smart filtering** - Respects `.gitignore`, excludes binary files, configurable patterns
- 📋 **Multiple outputs** - Stdout, clipboard, or file
- 🔧 **Flexible formats** - Markdown, XML, or plain text
- 📊 **Token estimation** - Know your context size before pasting
- 🤖 **Agent-friendly** - JSON output with structured errors and exit codes

## Installation

```bash
cargo install pctx
```

Or build from source:

```bash
git clone https://github.com/yourusername/pctx
cd pctx
cargo build --release
```

## Quick Start

```bash
# Generate context for current directory
pctx

# Copy to clipboard
pctx --clipboard

# Save to file
pctx --output context.md

# JSON output for scripts/agents
pctx --json

# Include specific files only
pctx --include "*.rs" --include "*.toml"

# Exclude test files
pctx --exclude "*.test.ts" --exclude "__tests__"
```

## Usage

### Basic Usage

```bash
# Current directory (default)
pctx

# Specific paths
pctx src/ lib/

# Single file
pctx src/main.rs
```

### Output Options

```bash
# To clipboard (requires clipboard feature)
pctx --clipboard

# To file (fails if exists)
pctx --output context.md

# To file (overwrite if exists)
pctx --output context.md --force

# Different formats
pctx --format markdown  # Default
pctx --format xml
pctx --format plain

# Include file tree
pctx --tree

# Include statistics
pctx --stats

# Use absolute paths instead of relative paths
pctx --absolute-paths
```

### Path Display

By default, pctx displays file paths relative to the current working directory:

```bash
# Default: relative paths
pctx
# Output shows: src/main.rs, src/lib.rs, etc.

# Use absolute paths
pctx --absolute-paths
# Output shows: /home/user/project/src/main.rs, etc.
```

### Filtering

```bash
# Exclude patterns (gitignore-style)
pctx --exclude "*.test.ts"
pctx --exclude "**/__tests__/**"

# Include only matching files
pctx --include "*.rs"
pctx --include "*.py" --include "*.pyi"

# Include hidden files
pctx --hidden

# Disable default exclusions
pctx --no-default-excludes

# Ignore .gitignore rules
pctx --no-gitignore

# Max file size (KB)
pctx --max-size 512
```

### Recursion Control

```bash
# Unlimited depth (default)
pctx --max-depth 0

# Only immediate children (no recursion)
pctx --max-depth 1

# Children and grandchildren only
pctx --max-depth 2
```

### Truncation

Large files are automatically truncated to keep context manageable:

```bash
# Max lines per file (0 = no limit)
pctx --max-lines 500

# Lines to keep at start/end when truncating
pctx --head-lines 20 --tail-lines 10

# Max characters per line
pctx --max-line-length 500 --head-chars 200 --tail-chars 100
```

### JSON Output (for Scripts/Agents)

```bash
# Full JSON response
pctx --json

# List files as JSON
pctx files list --json

# Error responses include structured information
pctx --json /nonexistent
# {
#   "status": "error",
#   "code": "file_not_found",
#   "message": "Directory not found: /nonexistent",
#   "suggestion": "Check that the path exists...",
#   "transient": false,
#   "exit_code": 3
# }
```

JSON output includes both relative and absolute paths:

```json
{
  "status": "success",
  "data": {
    "files": [
      {
        "path": "src/main.rs",
        "absolute_path": "/home/user/project/src/main.rs",
        "extension": "rs",
        "size_bytes": 1234,
        "line_count": 50,
        "truncated": false
      }
    ]
  }
}
```

### File Discovery

```bash
# List files that would be included
pctx files list

# Bare output for piping
pctx files list --quiet

# Show file tree
pctx files tree

# Combine with other tools
pctx files list --quiet | xargs wc -l
```

### Configuration

```bash
# Create config file
pctx config init

# Show current config
pctx config show

# Show default exclusions
pctx config defaults
```

#### `.pctx.toml` Example

```toml
# pctx configuration file
# See https://github.com/yourusername/pctx for documentation

# Additional patterns to exclude (gitignore-style)
exclude = [
    "*.test.ts",
    "*.spec.js",
    "__tests__",
    "*.snap",
]

# Only include files matching these patterns (empty = include all)
# include = ["*.rs", "*.toml"]

# Truncation settings for long files
# max_lines = 500      # Max lines per file (0 = no limit)
# head_lines = 20      # Lines to keep at start when truncating
# tail_lines = 10      # Lines to keep at end when truncating

# Truncation settings for long lines
# max_line_length = 500  # Max chars per line (0 = no limit)
# head_chars = 200       # Chars to keep at line start
# tail_chars = 100       # Chars to keep at line end
```

### Shell Completions

```bash
# Bash
pctx completions bash >> ~/.bashrc

# Zsh
pctx completions zsh >> ~/.zshrc

# Fish
pctx completions fish > ~/.config/fish/completions/pctx.fish
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General failure |
| 2 | Usage error (bad arguments) |
| 3 | File/directory not found |
| 4 | Permission denied |
| 5 | Conflict (file exists without --force) |
| 6 | No files matched filters |
| 7 | Partial success (some files failed) |

## Error Codes (JSON)

When using `--json`, errors include machine-readable codes:

- `file_not_found` - Path does not exist
- `permission_denied` - Cannot read file/directory
- `binary_file` - File is binary (skipped)
- `file_too_large` - File exceeds size limit
- `encoding_error` - File encoding issue
- `invalid_pattern` - Bad glob pattern
- `no_files_matched` - No files match filters
- `output_exists` - Output file exists (use --force)
- `config_error` - Configuration file error
- `clipboard_error` - Clipboard access failed

## Default Exclusions

pctx excludes common non-content directories and files by default:

- Version control: `.git`, `.svn`, `.hg`
- Dependencies: `node_modules`, `vendor`, `__pycache__`, `target`
- Build outputs: `dist`, `build`, `out`
- IDE files: `.idea`, `.vscode`
- Binary files: Images, archives, executables (detected automatically)
- Lock files: `package-lock.json`, `Cargo.lock`, etc.

Use `pctx config defaults` to see the full list, or `--no-default-excludes` to disable.

## Tips for LLM Context

1. **Be specific**: Use `--include` to focus on relevant files
2. **Watch token count**: Use `--stats` to see estimated tokens
3. **Truncate wisely**: Adjust `--max-lines` for large files
4. **Use tree view**: `--tree` helps LLMs understand project structure
5. **Iterate**: Use `--dry-run` to preview before generating

## Building

```bash
# Development build
cargo build

# Release build with all features
cargo build --release --all-features

# Without optional features
cargo build --release --no-default-features
```

### Features

- `clipboard` (default) - System clipboard support
- `tokens` (default) - Accurate token counting with tiktoken

## License

MIT
