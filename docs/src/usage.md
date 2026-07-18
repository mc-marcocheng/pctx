# Usage

## Quick start

```bash
# Generate context for the current directory
pctx

# Include a file tree
pctx --tree

# Copy generated context to the clipboard
pctx --clipboard

# Write generated context to a file
pctx --output context.md

# Overwrite an existing output file
pctx --output context.md --force

# Generate structured JSON
pctx --json

# Preview files without writing the final context
pctx --dry-run
```

## Command overview

```bash
# Generate context
pctx [OPTIONS] [PATHS...]

# List files that would be included
pctx files list [OPTIONS]

# Display the included file tree
pctx files tree [OPTIONS]

# Configuration management
pctx config show
pctx config init
pctx config defaults

# Shell completions
pctx completions bash
pctx completions zsh
pctx completions fish
pctx completions powershell
pctx completions elvish
```

If no path is supplied, `pctx` scans the current directory:

```bash
pctx
```

You can pass files or directories explicitly:

```bash
pctx src README.md Cargo.toml
```

## Output options

| Flag | Description |
|------|-------------|
| `--clipboard`, `-c` | Copy output to the system clipboard |
| `--output FILE`, `-o` | Write output to a file |
| `--force` | Overwrite the output file if it already exists |
| `--format FORMAT`, `-f` | Output format: `markdown`, `xml`, or `plain` |
| `--tree`, `-t` | Include a file tree at the beginning of the output |
| `--stats`, `-s` | Print a statistics summary |
| `--json` | Emit structured JSON |
| `--absolute-paths` | Display absolute paths instead of relative paths |
| `--stdin` | Read file paths from stdin, one per line |

Examples:

```bash
pctx --format markdown
pctx --format xml
pctx --format plain
pctx --tree --stats
pctx --absolute-paths
```

## Filtering options

| Flag | Description |
|------|-------------|
| `--exclude PATTERN`, `-e` | Exclude files matching a pattern; repeatable |
| `--include PATTERN`, `-i` | Include only files matching a pattern; repeatable |
| `--hidden` | Include dot-prefixed files and directories |
| `--no-default-excludes` | Disable built-in exclusion patterns |
| `--no-gitignore` | Ignore `.gitignore` rules |
| `--max-size KB` | Maximum file size to include, in KiB. Default: `1024` |
| `--max-depth N`, `-d` | Maximum traversal depth. `0` means unlimited |

Examples:

```bash
# Include only Rust and TOML files
pctx --include "*.rs" --include "*.toml"

# Exclude test files
pctx --exclude "*.test.ts" --exclude "__tests__"

# Disable built-in exclusions
pctx --no-default-excludes

# Ignore .gitignore files
pctx --no-gitignore

# Include files up to 2 MiB
pctx --max-size 2048

# Only scan immediate children
pctx --max-depth 1

# Scan children and grandchildren
pctx --max-depth 2
```

## Hidden files and directories

Dot-prefixed paths are hidden by default. Examples include:

- `.github`
- `.vscode`
- `.env`
- `project/.config/file.toml`

Use `--hidden` to include them:

```bash
pctx --hidden .github
pctx --hidden .github/workflows
```

Hidden-path filtering is separate from default exclusions and `.gitignore` rules.

This does **not** include `.github`:

```bash
pctx --no-default-excludes .github
```

Use:

```bash
pctx --hidden --no-default-excludes .github
```

The filtering layers are independent:

| Mechanism | Controlled by |
|-----------|---------------|
| Dot-prefixed paths | `--hidden` |
| Built-in exclusions such as `node_modules` and `target` | `--no-default-excludes` |
| `.gitignore` and related git ignore rules | `--no-gitignore` |

## Truncation options

| Flag | Description |
|------|-------------|
| `--no-truncation` | Disable file and line truncation |
| `--max-lines N` | Maximum lines per file before truncation. `0` means unlimited |
| `--head-lines N` | Lines to keep at the start of a truncated file |
| `--tail-lines N` | Lines to keep at the end of a truncated file |
| `--max-line-length N` | Maximum characters per line before truncation. `0` means unlimited |
| `--head-chars N` | Characters to keep at the start of a truncated line |
| `--tail-chars N` | Characters to keep at the end of a truncated line |

Defaults:

| Setting | Default |
|---------|---------|
| `max_lines` | `500` |
| `head_lines` | `20` |
| `tail_lines` | `10` |
| `max_line_length` | `500` |
| `head_chars` | `200` |
| `tail_chars` | `100` |

Examples:

```bash
# Keep more of long files
pctx --max-lines 1000 --head-lines 50 --tail-lines 25

# Disable only line-count truncation
pctx --max-lines 0

# Disable only long-line truncation
pctx --max-line-length 0

# Disable all truncation
pctx --no-truncation
```

When a file is truncated, pctx preserves the beginning and end of the file and inserts an omission marker between them.

## Stdin mode

Use `--stdin` to read paths from standard input:

```bash
find . -name "*.rs" -mtime -1 | pctx --stdin
```

This is useful for composing with other tools:

```bash
# Recently changed Rust files
find . -name "*.rs" -mtime -7 | pctx --stdin

# Files from a saved list
cat files_to_review.txt | pctx --stdin

# Files selected by pctx itself
pctx files list --quiet | grep -v test | pctx --stdin

# Changed files in git
git diff --name-only HEAD~5 | pctx --stdin

# Files found by fd
fd -e rs -e toml --changed-within 2weeks | pctx --stdin
```

Behavior in stdin mode:

- Empty lines are ignored.
- Whitespace around each line is trimmed.
- File paths are processed directly.
- Directory paths are expanded recursively.
- Positional paths are ignored when `--stdin` is used.
- Non-existent paths are reported as file errors; if some files succeed, the command exits with partial success.

## Listing files

```bash
# Human-readable file list
pctx files list

# Bare paths only, one per line
pctx files list --quiet

# JSON output
pctx files list --json
```

`--quiet` is designed for pipelines:

```bash
pctx files list --quiet | xargs wc -l
```

## File tree

```bash
pctx files tree
pctx files tree --json
```

To include the same tree in generated context:

```bash
pctx --tree
```

## Dry run

Use `--dry-run` to inspect what would be included:

```bash
pctx --dry-run
pctx --dry-run --json
```

Dry run still scans and processes files so it can report truncation and approximate token counts, but it does not write the final context document.

## Global options

| Flag | Description |
|------|-------------|
| `--json` | Use structured JSON output |
| `--verbose`, `-v` | Print additional diagnostics to stderr |
| `--quiet`, `-q` | Suppress non-essential output |
| `--no-color` | Disable colored output |

## Practical recipes

```bash
# Generate compact context for a Rust crate
pctx --include "*.rs" --include "*.toml" --tree

# Prepare context for a code review from changed files
git diff --name-only main...HEAD | pctx --stdin --tree

# Generate XML for a downstream parser
pctx --format xml --output context.xml

# Copy only source-like files to clipboard
pctx src Cargo.toml README.md --clipboard

# Show what would be included after filters
pctx --include "*.ts" --exclude "*.test.ts" --dry-run

# Find files pctx would include, then post-filter with grep
pctx files list --quiet | grep -E '\.(rs|toml)$' | pctx --stdin
```
