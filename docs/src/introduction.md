# Introduction

**pctx** generates LLM-ready context from your codebase. It intelligently packages source files with proper formatting, truncation, and filtering for optimal AI assistant consumption.

## Motivation

When working with modern AI coding harnesses, agents often need to make multiple, sequential tool calls just to explore, read, and understand the layout of your project. This back-and-forth communication is slow, consumes excess tokens for system overhead, and can easily derail an agent's train of thought before it even starts writing code.

**pctx** solves this by providing a unified, pre-packaged snapshot of your project's context. By generating a single, intelligently filtered and truncated document, you can:
- **Reduce Latency**: Eliminate the need for the AI to "poke around" your filesystem using search and read commands.
- **Improve Accuracy**: Provide the AI with an immediate, holistic view of the project's structure and relevant files.
- **Save Tokens**: Filter out noise (like binaries, build artifacts, and vendor directories) and smartly truncate long files so you only pay for the context that matters.
- **Maintain Control**: Keep sensitive or irrelevant information out of the context window using standard gitignore syntax.

## Features

- **Smart file discovery**: Respects `.gitignore`, excludes binary files, and filters common non-source directories
- **Multiple output formats**: Markdown (default), XML, and plain text
- **Intelligent truncation**: Preserves file head and tail when truncating large files
- **Flexible filtering**: Include/exclude patterns with gitignore-style syntax
- **Multiple destinations**: stdout, clipboard, or file output
- **JSON mode**: Structured output for programmatic use and CI/CD integration
- **Stdin support**: Read file lists from pipes for integration with other tools
- **Token estimation**: Approximate token counts for various LLM models
