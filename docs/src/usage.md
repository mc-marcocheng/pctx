# Usage

## Quick Start

```bash
# Generate context for current directory
pctx

# Copy to clipboard
pctx --clipboard

# Write to file
pctx --output context.md

# JSON output for scripts
pctx --json

# Filter specific files
pctx --include "*.rs" --include "*.toml"
pctx --exclude "*.test.ts" --exclude "__tests__"

# Pipe file list from other tools
find . -name "*.rs" -mtime -7 | pctx --stdin
pctx files list --quiet | grep -v test | pctx --stdin

# Preview without generating
pctx --dry-run

# Include file tree in output
pctx --tree

# Disable truncation for full file contents
pctx --no-truncation
```

## Basic Commands

```bash
# Default: generate context from current directory
pctx [OPTIONS] [PATHS...]

# List files that would be included
pctx files list [OPTIONS]

# Show file tree structure
pctx files tree [OPTIONS]

# Configuration management
pctx config show      # Show current config
pctx config init      # Create .pctx.toml
pctx config defaults  # List default excludes

# Generate shell completions
pctx completions bash
pctx completions zsh
pctx completions fish
```

## Output Options

| Flag | Description |
|------|-------------|
| `--clipboard`, `-c` | Copy output to system clipboard |
| `--output FILE`, `-o` | Write to file (use `--force` to overwrite) |
| `--format`, `-f` | Output format: `markdown`, `xml`, `plain` |
| `--tree`, `-t` | Include file tree at beginning of output |
| `--stats`, `-s` | Show statistics summary |
| `--json` | Structured JSON output (for scripts) |
| `--stdin` | Read file paths from stdin (one per line) |

## Filtering Options

| Flag | Description |
|------|-------------|
| `--exclude PATTERN`, `-e` | Exclude files matching pattern (repeatable) |
| `--include PATTERN`, `-i` | Include only files matching pattern (repeatable) |
| `--hidden` | Include hidden files (starting with `.`) |
| `--no-default-excludes` | Disable built-in exclusions |
| `--no-gitignore` | Ignore `.gitignore` rules |
| `--max-size KB` | Maximum file size in KB (default: 1024) |
| `--max-depth N`, `-d` | Limit directory recursion depth |

## Truncation Options

| Flag | Description |
|------|-------------|
| `--no-truncation` | Disable all truncation |
| `--max-lines N` | Max lines per file before truncating (default: 500, 0 = unlimited) |
| `--head-lines N` | Lines to keep at file start (default: 20) |
| `--tail-lines N` | Lines to keep at file end (default: 10) |
| `--max-line-length N` | Max chars per line (default: 500, 0 = unlimited) |
| `--head-chars N` | Chars to keep at line start (default: 200) |
| `--tail-chars N` | Chars to keep at line end (default: 100) |

## Stdin Mode

The `--stdin` flag allows reading file paths from standard input, enabling powerful integrations:

```bash
# Process only recently modified Rust files
find . -name "*.rs" -mtime -1 | pctx --stdin

# Process files from a list
cat files_to_review.txt | pctx --stdin

# Chain with pctx's own file listing
pctx files list --quiet | grep -v _test | pctx --stdin

# Use with git to process only changed files
git diff --name-only HEAD~5 | pctx --stdin

# Process files matching complex criteria
fd -e rs -e toml --changed-within 2weeks | pctx --stdin
```

When using `--stdin`:
- Empty lines and whitespace-only lines are ignored
- Non-existent files are skipped with a warning (in verbose mode)
- Directories in the input are expanded recursively
