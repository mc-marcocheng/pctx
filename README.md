# pctx

<p align="center">
  <img src="https://github.com/mc-marcocheng/pctx/blob/master/assets/pctx-carbon.png?raw=true" alt="pctx demo" width="600">
</p>

<p align="center">
  <a href="https://crates.io/crates/pctx">
    <img src="https://img.shields.io/crates/v/pctx.svg" alt="Crates.io version">
  </a>
  <a href="https://opensource.org/licenses/MIT">
    <img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT">
  </a>
  <a href="https://mc-marcocheng.github.io/pctx">
    <img src="https://img.shields.io/badge/docs-mdBook-blue.svg" alt="Documentation">
  </a>
</p>

Generate LLM-ready context from your codebase. Intelligently packages source files with proper formatting, truncation, and filtering for optimal AI assistant consumption.

## Features

- **Smart file discovery**: Respects `.gitignore`, excludes binary files, and filters common non-source directories
- **Multiple output formats**: Markdown (default), XML, and plain text
- **Intelligent truncation**: Preserves file head and tail when truncating large files
- **Flexible filtering**: Include/exclude patterns with gitignore-style syntax
- **Multiple destinations**: stdout, clipboard, or file output
- **JSON mode**: Structured output for programmatic use and CI/CD integration
- **Stdin support**: Read file lists from pipes for integration with other tools
- **Token estimation**: Approximate token counts for various LLM models

## Installation

```bash
cargo install pctx
```

## Quick Start

```bash
# Generate context for current directory
pctx

# Copy to clipboard
pctx --clipboard

# Write to file
pctx --output context.md
```

See the [Documentation](https://mc-marcocheng.github.io/pctx) for advanced usage, filtering, and configuration options.
