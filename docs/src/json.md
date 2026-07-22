# JSON & Exit Codes

Use `--json` for machine-readable output:

```bash
pctx --json
```

In JSON mode:

- The structured response is written to stdout.
- Progress and diagnostic messages are written to stderr.
- Errors are also written to stdout as JSON.
- The process exit code still indicates success or failure.
- Requested side effects (`--output FILE`, `--clipboard`) are completed before the JSON response is printed, so a side-effect failure produces a single JSON error response rather than a success response followed by an error.

## Capabilities

Integrations that need to detect which features a given `pctx` build supports can query them in a stable, machine-readable form:

```bash
pctx --json capabilities
```

```json
{
  "schema_version": 1,
  "name": "pctx",
  "version": "1.1.0",
  "clipboard": true,
  "tokens": true,
  "json_output": true,
  "stdin": true,
  "stdin0": true,
  "paths_file0": true,
  "path_aliases": true,
  "formats": ["markdown", "xml", "plain"]
}
```

## Response statuses

Every JSON response has a top-level `status`.

Possible values:

| Status | Meaning |
|--------|---------|
| `success` | Operation completed successfully |
| `partial` | Some files were processed, but some failed or were skipped with errors |
| `error` | The operation failed |

## Successful context response

```bash
pctx --json
```

Example shape:

```json
{
  "status": "success",
  "data": {
    "content": "`src/lib.rs`:\n```rust\npub mod cli;\n```\n",
    "format": "markdown",
    "files": [
      {
        "path": "src/lib.rs",
        "extension": "rs",
        "size_bytes": 128,
        "line_count": 8,
        "truncated": false
      }
    ]
  },
  "stats": {
    "file_count": 1,
    "total_lines": 8,
    "total_bytes": 128,
    "truncated_count": 0,
    "skipped_count": 0,
    "token_estimate": 42,
    "duration_ms": 3
  }
}
```

## Partial response

If some files fail but others succeed, pctx returns `partial` and exits with code `7`.

Example:

```json
{
  "status": "partial",
  "data": {
    "content": "...",
    "format": "markdown",
    "files": [
      {
        "path": "src/lib.rs",
        "extension": "rs",
        "size_bytes": 128,
        "line_count": 8,
        "truncated": false
      }
    ]
  },
  "stats": {
    "file_count": 1,
    "total_lines": 8,
    "total_bytes": 128,
    "truncated_count": 0,
    "skipped_count": 0,
    "token_estimate": 42,
    "duration_ms": 3
  },
  "errors": [
    {
      "path": "large.log",
      "code": "file_too_large",
      "message": "File too large (2000000 bytes, max 1048576): large.log",
      "transient": false
    }
  ]
}
```

## Error response

Example:

```json
{
  "status": "error",
  "code": "no_files_matched",
  "message": "No files matched the specified filters",
  "input": {
    "paths": [],
    "exclude": [],
    "include": ["*.rs"],
    "hidden": false,
    "no_default_excludes": false,
    "no_gitignore": false,
    "max_size_kb": 1024,
    "max_depth": 0,
    "stdin": false
  },
  "suggestion": "Include patterns are active; check whether the files match `--include` or the `include` entries in `.pctx.toml`.",
  "transient": false,
  "exit_code": 6
}
```

## File list JSON

```bash
pctx files list --json
```

Example shape:

```json
{
  "status": "success",
  "data": [
    {
      "path": "src/lib.rs",
      "extension": "rs",
      "size_bytes": 128,
      "truncated": false
    }
  ],
  "stats": {
    "file_count": 1,
    "total_lines": 0,
    "total_bytes": 0,
    "truncated_count": 0,
    "skipped_count": 0,
    "duration_ms": 0
  }
}
```

`files list` does not read file contents, so `line_count` is omitted.

## Tree JSON

```bash
pctx files tree --json
```

Example shape:

```json
{
  "status": "success",
  "data": {
    "tree": "src\n└── lib.rs\n"
  },
  "stats": {
    "file_count": 1,
    "total_lines": 0,
    "total_bytes": 0,
    "truncated_count": 0,
    "skipped_count": 0,
    "duration_ms": 0
  }
}
```

## Error codes

Common machine-readable error codes include:

| Code | Meaning |
|------|---------|
| `file_not_found` | File or directory does not exist |
| `permission_denied` | File or directory could not be read |
| `binary_file` | File appears to be binary |
| `file_too_large` | File exceeds `--max-size` |
| `encoding_error` | File encoding could not be handled |
| `invalid_pattern` | Include/exclude pattern is invalid |
| `no_files_matched` | Filters matched no files |
| `output_exists` | Output file exists and `--force` was not used |
| `git_error` | Git command failed |
| `config_error` | Config file could not be parsed or used |
| `clipboard_error` | Clipboard write failed |
| `io_error` | Generic I/O error |
| `json_error` | JSON serialization failed |
| `walk_error` | Directory traversal failed |
| `ignore_error` | Ignore-pattern handling failed |

## Exit codes

Exit codes are part of the CLI contract.

| Exit code | Name | Meaning |
|-----------|------|---------|
| `0` | Success | Operation completed successfully |
| `1` | Failure | General or unspecified failure |
| `2` | Usage error | Invalid arguments or bad flag combinations |
| `3` | Not found | File, directory, or config file not found |
| `4` | Permission denied | Cannot read a file or directory |
| `5` | Conflict | Output file exists without `--force` |
| `6` | No match | No files matched filters |
| `7` | Partial | Some files succeeded and some failed |

## Scripting examples

```bash
# Extract generated context
pctx --json | jq -r '.data.content'

# Get included file paths
pctx --json | jq -r '.data.files[].path'

# Fail on partial success
response="$(pctx --json)"
status="$(printf '%s' "$response" | jq -r '.status')"
test "$status" = "success"

# List large files pctx would include
pctx files list --json \
  | jq -r '.data[] | select(.size_bytes > 10000) | .path'

# Use exit codes
if pctx --json > context.json; then
  echo "success"
else
  case "$?" in
    6) echo "no files matched" ;;
    7) echo "partial success" ;;
    *) echo "failed" ;;
  esac
fi
```
