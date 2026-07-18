# Introduction

**pctx** generates LLM-ready context from your codebase.

It scans files, applies sensible filters, truncates oversized content, and formats the result as Markdown, XML, plain text, or structured JSON. The output is designed to be pasted into, copied to, or consumed by AI coding assistants.

## Why pctx?

AI coding agents often spend many tool calls discovering a project:

1. list files
2. inspect directories
3. read files
4. skip build artifacts
5. recover from oversized or binary files
6. repeat

That exploration costs time and tokens. **pctx** packages the useful parts of a repository into one controlled context snapshot.

Use it to:

- **Reduce latency** by avoiding repeated filesystem exploration.
- **Improve accuracy** by giving an assistant a broad project view up front.
- **Save tokens** with default exclusions and truncation.
- **Control scope** with include/exclude patterns and `.pctx.toml`.
- **Automate workflows** with JSON output and stable exit codes.

## What pctx outputs

By default, `pctx` writes Markdown to stdout:

````markdown
`src/lib.rs`:
```rust
pub mod cli;
pub mod config;
```

`README.md`:
```markdown
# My Project
```
````

You can also include a file tree:

```bash
pctx --tree
```

Or produce structured output for scripts:

```bash
pctx --json
```

## Features

- Recursive file discovery
- `.gitignore` support
- Built-in exclusions for common noisy paths
- Hidden-path filtering with explicit `--hidden` opt-in
- Binary-file detection
- File size limits
- Include/exclude patterns using gitignore-style syntax
- Markdown, XML, and plain text output
- Clipboard and file destinations
- JSON output for automation
- Stdin mode for integrating with `find`, `fd`, `git diff`, and other tools
- Truncation for long files and long lines
- Approximate token estimation

## Safety note

`pctx` reads local files and emits their contents. Review the generated context before sharing it with external services.

Default exclusions skip many common secrets and environment files, such as `.env`, `.env.*`, `*.pem`, and `*.key`, but no automatic filter can guarantee that sensitive information is excluded.
