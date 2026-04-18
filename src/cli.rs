//! Command-line interface definitions using clap.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "pctx",
    version,
    author,
    about = "Generate LLM-ready context from your codebase",
    long_about = r#"Generate LLM-ready context from your codebase.

STRUCTURED OUTPUT:
  Use --json for machine-readable output. All progress messages go to stderr,
  only the result goes to stdout. Exit codes indicate success/failure type.

  Note: In --json mode, errors are output to stdout as part of the JSON response
  to maintain a consistent API contract for programmatic consumers.

RECURSION:
  By default, pctx recursively scans all directories. Use --max-depth to limit:
    --max-depth 1    Only immediate children (no recursion)
    --max-depth 2    Children and grandchildren
    --max-depth 0    Unlimited depth (default)

EXIT CODES:
  0  Success
  1  General failure
  2  Usage error (bad arguments)
  3  File/directory not found
  4  Permission denied
  5  Conflict (file exists)
  6  No files matched filters
  7  Partial success (some files failed)

EXAMPLES:
  # Basic usage - current directory to stdout
  pctx

  # JSON output for programmatic use
  pctx --json

  # Copy to clipboard
  pctx --clipboard

  # Write to file (fails if exists, use --force to overwrite)
  pctx --output context.md
  pctx --output context.md --force

  # Filter files
  pctx --exclude "*.test.ts" --exclude "__tests__"
  pctx --include "*.rs" --include "*.toml"

  # Read file list from stdin
  find . -name "*.rs" | pctx --stdin
  pctx files list --quiet | pctx --stdin

  # List files without generating context
  pctx files list --json

  # Quiet mode - just file paths, one per line (for piping)
  pctx files list --quiet

  # Adjust truncation thresholds
  pctx --max-lines 1000 --head-lines 50 --tail-lines 20

  # Disable all truncation
  pctx --no-truncation

  # Limit recursion depth
  pctx --max-depth 2

  # Dry run with full preview
  pctx --dry-run --json

  # Pipe-friendly: get paths of large files
  pctx files list --json | jq -r '.data[] | select(.size_bytes > 10000) | .path'"#,
    after_help = "For more information, visit: https://github.com/mc-marcocheng/pctx"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(flatten)]
    pub generate: GenerateArgs,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// File discovery and listing operations
    #[command(subcommand)]
    Files(FilesCommands),

    /// Configuration management
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand, Debug)]
pub enum FilesCommands {
    /// List files that would be included in context
    #[command(
        long_about = "List all files that would be included in the context.\n\n\
                      Use --json for structured output, --quiet for bare paths (one per line).\n\n\
                      EXAMPLES:\n  \
                      pctx files list                    # Human-readable list\n  \
                      pctx files list --json             # JSON array of file info\n  \
                      pctx files list --quiet            # Bare paths for piping\n  \
                      pctx files list -q | xargs wc -l   # Count lines in each file"
    )]
    List {
        #[command(flatten)]
        filter: FilterArgs,

        /// Output bare file paths only, one per line (for piping)
        #[arg(short, long)]
        quiet: bool,
    },

    /// Display file tree structure
    #[command(
        long_about = "Show the tree structure of files that would be included.\n\n\
                      EXAMPLES:\n  \
                      pctx files tree                    # Visual tree\n  \
                      pctx files tree --json             # JSON tree structure"
    )]
    Tree {
        #[command(flatten)]
        filter: FilterArgs,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Show the resolved configuration
    #[command(
        long_about = "Display the current configuration after merging defaults, \
                            config file, and command-line options."
    )]
    Show,

    /// Create a new .pctx.toml configuration file
    #[command(
        long_about = "Initialize a new .pctx.toml config file in the current directory.\n\n\
                      EXAMPLES:\n  \
                      pctx config init                   # Create config (fails if exists)\n  \
                      pctx config init --force           # Overwrite existing config"
    )]
    Init {
        /// Overwrite existing config file if it exists
        #[arg(long)]
        force: bool,
    },

    /// Show default exclusion patterns
    #[command(long_about = "List all patterns that are excluded by default \
                            (node_modules, .git, etc.)")]
    Defaults,
}

/// Global arguments available to all commands
#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    /// Output as JSON (structured output to stdout, messages to stderr)
    #[arg(long, global = true)]
    pub json: bool,

    /// Enable verbose output (to stderr)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all non-essential output
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Path to config file [default: .pctx.toml in current or parent dirs]
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,
}

/// Arguments for the main generate command (default when no subcommand)
#[derive(Args, Debug, Clone)]
pub struct GenerateArgs {
    /// Paths to include (files or directories)
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    #[command(flatten)]
    pub filter: FilterArgs,

    #[command(flatten)]
    pub output: OutputArgs,

    #[command(flatten)]
    pub truncation: TruncationArgs,

    /// Preview what would be generated without producing output
    #[arg(long)]
    pub dry_run: bool,

    /// Model name for token estimation
    #[arg(long, value_name = "MODEL", default_value = "gpt-4")]
    pub token_model: String,

    /// Read file paths from stdin (one per line)
    #[arg(long)]
    pub stdin: bool,
}

/// Arguments for filtering which files to include
#[derive(Args, Debug, Clone, Default)]
pub struct FilterArgs {
    /// Exclude files matching pattern (gitignore-style, repeatable)
    #[arg(short, long = "exclude", value_name = "PATTERN")]
    pub exclude: Vec<String>,

    /// Include only files matching pattern (gitignore-style, repeatable)
    #[arg(short, long = "include", value_name = "PATTERN")]
    pub include: Vec<String>,

    /// Include hidden files and directories (starting with .)
    #[arg(long)]
    pub hidden: bool,

    /// Disable default exclusion patterns
    #[arg(long)]
    pub no_default_excludes: bool,

    /// Ignore .gitignore rules
    #[arg(long)]
    pub no_gitignore: bool,

    /// Maximum file size to include (in KB)
    #[arg(long, default_value = "1024", value_name = "KB")]
    pub max_size: u64,

    /// Maximum directory traversal depth (0 = unlimited, 1 = no recursion)
    #[arg(short = 'd', long, default_value = "0", value_name = "N")]
    pub max_depth: usize,
}

/// Arguments controlling output destination and format
#[derive(Args, Debug, Clone)]
pub struct OutputArgs {
    /// Copy output to system clipboard
    #[arg(short, long)]
    pub clipboard: bool,

    /// Write output to file (use --force to overwrite)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Overwrite output file if it already exists
    #[arg(long, requires = "output")]
    pub force: bool,

    /// Content format for the generated context
    #[arg(short, long, value_enum, default_value = "markdown")]
    pub format: ContentFormat,

    /// Include a file tree at the beginning of output
    #[arg(short, long)]
    pub tree: bool,

    /// Include statistics summary in output
    #[arg(short, long)]
    pub stats: bool,

    /// Display absolute paths instead of relative paths
    #[arg(long)]
    pub absolute_paths: bool,
}

/// Arguments for content truncation thresholds
#[derive(Args, Debug, Clone)]
pub struct TruncationArgs {
    /// Disable all truncation (equivalent to --max-lines 0 --max-line-length 0)
    #[arg(long, conflicts_with_all = ["max_lines", "max_line_length"])]
    pub no_truncation: bool,

    /// Maximum lines per file before truncation (0 = no limit)
    #[arg(long, default_value = "500", value_name = "N")]
    pub max_lines: usize,

    /// Number of lines to keep at the start when truncating
    #[arg(long, default_value = "20", value_name = "N")]
    pub head_lines: usize,

    /// Number of lines to keep at the end when truncating
    #[arg(long, default_value = "10", value_name = "N")]
    pub tail_lines: usize,

    /// Maximum characters per line before truncation (0 = no limit)
    #[arg(long, default_value = "500", value_name = "N")]
    pub max_line_length: usize,

    /// Characters to keep at line start when truncating
    #[arg(long, default_value = "200", value_name = "N")]
    pub head_chars: usize,

    /// Characters to keep at line end when truncating
    #[arg(long, default_value = "100", value_name = "N")]
    pub tail_chars: usize,
}

impl TruncationArgs {
    /// Check if max_lines was explicitly set (not default)
    pub fn max_lines_explicit(&self) -> bool {
        // This is a workaround - ideally we'd use Option<usize>
        // For now, we check if no_truncation is set
        !self.no_truncation
    }

    /// Get effective max_lines value
    pub fn effective_max_lines(&self) -> usize {
        if self.no_truncation {
            0
        } else {
            self.max_lines
        }
    }

    /// Get effective max_line_length value
    pub fn effective_max_line_length(&self) -> usize {
        if self.no_truncation {
            0
        } else {
            self.max_line_length
        }
    }
}

/// Supported content output formats
#[derive(ValueEnum, Clone, Debug, Default, PartialEq)]
pub enum ContentFormat {
    /// Markdown with fenced code blocks
    #[default]
    Markdown,
    /// XML tags wrapping content
    Xml,
    /// Plain text with simple separators
    Plain,
}

/// Supported shells for completion generation
#[derive(ValueEnum, Clone, Debug)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}
