# Configuration

## Config File

Create a `.pctx.toml` file in your project root:

```bash
pctx config init
```

Example configuration:

```toml
# Patterns to exclude (in addition to defaults)
exclude = [
    "*.generated.ts",
    "vendor/",
    "__snapshots__",
]

# Patterns to include (if specified, only these are included)
include = [
    "*.rs",
    "*.toml",
]

# Truncation settings
max_lines = 500
head_lines = 20
tail_lines = 10
max_line_length = 500
```

Configuration is loaded from `.pctx.toml` in the current directory or any parent directory. If the config file exists but has syntax errors, a warning is printed and the file is skipped.

## Configuration Precedence

Settings are applied in this order (highest priority first):

1. Command-line arguments
2. Config file (`.pctx.toml`)
3. Built-in defaults

## Default Exclusions

Common directories and files are excluded by default:

- **Version control**: `.git`, `.svn`, `.hg`
- **Dependencies**: `node_modules`, `vendor`, `target`, `.venv`
- **Build outputs**: `dist`, `build`, `out`, `bin`, `obj`
- **IDE/Editor**: `.idea`, `.vscode`, `.vs`
- **Caches**: `__pycache__`, `.cache`, `.pytest_cache`
- **Lock files**: `package-lock.json`, `yarn.lock`, `Cargo.lock`, etc.

See all defaults with: `pctx config defaults`

## Pattern Syntax

Patterns follow gitignore-style syntax:

| Pattern | Matches |
|---------|---------|
| `*.log` | All `.log` files |
| `test_*` | Files starting with `test_` |
| `**/tests/**` | Any `tests` directory at any level |
| `/src/generated` | `src/generated` at root only |
| `docs/` | `docs` directory |

**Limitations:**
- Negation patterns (`!pattern`) are not supported and will show a warning
- Character classes (`[abc]`) depend on glob crate support
- Some edge cases with `**/` patterns may differ from git behavior
