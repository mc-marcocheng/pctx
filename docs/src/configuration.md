# Configuration

`pctx` can load project defaults from a `.pctx.toml` file.

Create one with:

```bash
pctx config init
```

By default, pctx searches for `.pctx.toml` in the current directory and then in parent directories.

## Supported keys

The configuration file currently supports:

| Key | Type | Description |
|-----|------|-------------|
| `exclude` | array of strings | Additional gitignore-style exclusion patterns |
| `include` | array of strings | If non-empty, only matching files are included |
| `max_lines` | integer | Maximum lines per file before truncation. `0` means unlimited |
| `head_lines` | integer | Lines to keep at the start of a truncated file |
| `tail_lines` | integer | Lines to keep at the end of a truncated file |
| `max_line_length` | integer | Maximum characters per line before truncation. `0` means unlimited |
| `head_chars` | integer | Characters to keep at the start of a truncated line |
| `tail_chars` | integer | Characters to keep at the end of a truncated line |

Example:

```toml
# Patterns to exclude in addition to built-in defaults
exclude = [
    "*.generated.ts",
    "vendor/",
    "__snapshots__",
]

# If specified, only matching files are included
include = [
    "*.rs",
    "*.toml",
    "*.md",
]

# File truncation
max_lines = 500
head_lines = 20
tail_lines = 10

# Long-line truncation
max_line_length = 500
head_chars = 200
tail_chars = 100
```

## Precedence

Settings are applied in this order, highest priority first:

1. Command-line arguments
2. `.pctx.toml`
3. Built-in defaults

For example:

```toml
max_lines = 500
```

can be overridden with:

```bash
pctx --max-lines 1000
```

Include and exclude patterns are additive:

1. Built-in excludes are added first, unless `--no-default-excludes` is used.
2. Config-file excludes are added.
3. CLI excludes are added.
4. Config-file includes are added.
5. CLI includes are added.

## Config commands

```bash
# Show the resolved config file contents or defaults
pctx config show

# Create .pctx.toml in the current directory
pctx config init

# Overwrite an existing .pctx.toml
pctx config init --force

# Print built-in exclusion patterns
pctx config defaults
```

JSON is also supported:

```bash
pctx config show --json
pctx config defaults --json
```

## Syntax errors

During normal context generation, if a discovered `.pctx.toml` exists but cannot be parsed, pctx prints a warning and continues without that config file.

`pctx config show` is stricter: parse errors are reported as command errors.

## Default exclusions

pctx excludes many common noisy or unsafe files by default.

Examples include:

- **Version control**: `.git`, `.svn`, `.hg`
- **Dependencies**: `node_modules`, `vendor`, `bower_components`
- **Rust**: `target`, `Cargo.lock`
- **Python**: `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.venv`
- **Build outputs**: `dist`, `build`, `out`, `bin`, `obj`
- **Editor files**: `.idea`, `.vscode`, `*.swp`
- **Caches**: `.cache`, `.parcel-cache`, `.turbo`, `.next`
- **Environment/secrets**: `.env`, `.env.*`, `*.pem`, `*.key`
- **Logs**: `*.log`, `logs`
- **Media and binaries**: `*.png`, `*.jpg`, `*.pdf`, `*.zip`, `*.dll`, `*.so`
- **Generated/minified files**: `*.map`, `*.min.js`, `*.min.css`

View the exact list:

```bash
pctx config defaults
```

Disable built-in exclusions:

```bash
pctx --no-default-excludes
```

## Hidden paths are separate

Hidden-path filtering is not part of the default exclusion pattern list.

Dot-prefixed paths such as `.github` require `--hidden`, even when:

- `--no-default-excludes` is used
- `--no-gitignore` is used
- `.gitignore` does not exclude them

Example:

```bash
# Usually no files matched, because .github is hidden
pctx --no-default-excludes .github

# Include it explicitly
pctx --hidden --no-default-excludes .github
```

## Pattern syntax

Patterns use gitignore-style matching.

| Pattern | Matches |
|---------|---------|
| `*.log` | `.log` files at any level |
| `test_*` | Files or path components starting with `test_` |
| `**/tests/**` | Any `tests` directory at any level |
| `/src/generated` | `src/generated` at the scan root |
| `docs/` | A directory named `docs` |
| `src/config` | `src/config` and files below it |
| `src/config/` | Files below a directory named `src/config` |

Examples:

```toml
exclude = [
    "*.log",
    "node_modules",
    "dist/",
    "**/*.generated.ts",
]

include = [
    "*.rs",
    "src/**/*.toml",
]
```

## Pattern limitations

- Negation patterns such as `!important.log` are not supported.
- Unsupported negation patterns are ignored with a warning.
- Character-class behavior such as `[abc]` depends on the underlying glob implementation.
- Some `**/` edge cases may differ from exact git behavior.
- Leading `./` or `.\` is stripped with a warning because include/exclude values are patterns, not positional paths.

Prefer positional paths when selecting a concrete directory:

```bash
# Good: scan this path
pctx src/config

# Also valid: pattern filtering
pctx --include "src/config"
```

## Troubleshooting

### No files matched

Try:

```bash
pctx --dry-run --verbose
```

Common causes:

- The path is dot-prefixed and needs `--hidden`.
- Include patterns are too restrictive.
- A custom exclude pattern matched more than expected.
- Built-in exclusions filtered the files.
- `.gitignore` filtered the files.
- Files exceeded `--max-size`.
- Files are binary.

### A hidden directory is still skipped

Use `--hidden`:

```bash
pctx --hidden .github
```

`--no-default-excludes` does not affect hidden-path filtering.

### A config pattern did not behave like git

pctx supports gitignore-style patterns, but not every gitignore feature. Avoid negation patterns and test with:

```bash
pctx files list --dry-run
```

For file listing, use:

```bash
pctx files list --verbose
```
