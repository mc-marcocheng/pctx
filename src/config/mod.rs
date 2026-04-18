//! Configuration handling for pctx.
//!
//! Configuration is built from multiple sources in order of precedence:
//! 1. Command-line arguments (highest)
//! 2. Config file (.pctx.toml)
//! 3. Built-in defaults (lowest)

pub mod defaults;
pub mod file;

use std::path::PathBuf;

use crate::cli::{ContentFormat, FilterArgs, GenerateArgs, GlobalArgs, TruncationArgs};
use crate::error::PctxError;

/// Truncation settings for long files and lines
#[derive(Debug, Clone)]
pub struct TruncationConfig {
    pub max_lines: usize,
    pub head_lines: usize,
    pub tail_lines: usize,
    pub max_line_length: usize,
    pub head_chars: usize,
    pub tail_chars: usize,
}

impl Default for TruncationConfig {
    fn default() -> Self {
        Self {
            max_lines: 500,
            head_lines: 20,
            tail_lines: 10,
            max_line_length: 500,
            head_chars: 200,
            tail_chars: 100,
        }
    }
}

impl From<&TruncationArgs> for TruncationConfig {
    fn from(args: &TruncationArgs) -> Self {
        Self {
            max_lines: args.max_lines,
            head_lines: args.head_lines,
            tail_lines: args.tail_lines,
            max_line_length: args.max_line_length,
            head_chars: args.head_chars,
            tail_chars: args.tail_chars,
        }
    }
}

/// Complete resolved configuration for a pctx operation
#[derive(Debug, Clone)]
pub struct Config {
    pub paths: Vec<PathBuf>,
    pub exclude_patterns: Vec<String>,
    pub include_patterns: Vec<String>,
    pub include_hidden: bool,
    pub use_default_excludes: bool,
    pub use_gitignore: bool,
    pub max_file_size: u64,
    pub max_depth: Option<usize>,
    pub truncation: TruncationConfig,
    pub output_format: ContentFormat,
    pub show_tree: bool,
    pub show_stats: bool,
    pub absolute_paths: bool,
    pub verbose: bool,
    pub quiet: bool,
}

impl Config {
    /// Build configuration from generate command arguments
    pub fn from_args(args: &GenerateArgs, global: &GlobalArgs) -> Result<Self, PctxError> {
        let file_config = file::find_and_load().ok();

        let (exclude_patterns, include_patterns) =
            Self::build_patterns(&args.filter, file_config.as_ref());

        // Merge truncation settings
        let truncation = Self::build_truncation(&args.truncation, file_config.as_ref());

        Ok(Self {
            paths: args.paths.clone(),
            exclude_patterns,
            include_patterns,
            include_hidden: args.filter.hidden,
            use_default_excludes: !args.filter.no_default_excludes,
            use_gitignore: !args.filter.no_gitignore,
            max_file_size: args.filter.max_size * 1024,
            max_depth: if args.filter.max_depth == 0 {
                None
            } else {
                Some(args.filter.max_depth)
            },
            truncation,
            output_format: args.output.format.clone(),
            show_tree: args.output.tree,
            show_stats: args.output.stats,
            absolute_paths: args.output.absolute_paths,
            verbose: global.verbose,
            quiet: global.quiet,
        })
    }

    /// Build configuration from filter arguments only (for subcommands)
    pub fn from_filter_args(filter: &FilterArgs, global: &GlobalArgs) -> Result<Self, PctxError> {
        let file_config = file::find_and_load().ok();

        let (exclude_patterns, include_patterns) =
            Self::build_patterns(filter, file_config.as_ref());

        Ok(Self {
            paths: vec![PathBuf::from(".")],
            exclude_patterns,
            include_patterns,
            include_hidden: filter.hidden,
            use_default_excludes: !filter.no_default_excludes,
            use_gitignore: !filter.no_gitignore,
            max_file_size: filter.max_size * 1024,
            max_depth: if filter.max_depth == 0 {
                None
            } else {
                Some(filter.max_depth)
            },
            truncation: TruncationConfig::default(),
            output_format: ContentFormat::default(),
            show_tree: false,
            show_stats: false,
            absolute_paths: false,
            verbose: global.verbose,
            quiet: global.quiet,
        })
    }

    /// Build exclude and include patterns from filter args and file config
    fn build_patterns(
        filter: &FilterArgs,
        file_config: Option<&file::FileConfig>,
    ) -> (Vec<String>, Vec<String>) {
        // Build exclude patterns
        let mut exclude_patterns = if filter.no_default_excludes {
            Vec::new()
        } else {
            defaults::DEFAULT_EXCLUDES
                .iter()
                .map(|s| s.to_string())
                .collect()
        };

        // Add patterns from config file
        if let Some(fc) = file_config {
            exclude_patterns.extend(fc.exclude.clone());
        }

        // Add patterns from command line (highest priority)
        exclude_patterns.extend(filter.exclude.clone());

        // Build include patterns
        let mut include_patterns = Vec::new();
        if let Some(fc) = file_config {
            include_patterns.extend(fc.include.clone());
        }
        include_patterns.extend(filter.include.clone());

        (exclude_patterns, include_patterns)
    }

    /// Build truncation config from args and file config
    fn build_truncation(
        args: &TruncationArgs,
        file_config: Option<&file::FileConfig>,
    ) -> TruncationConfig {
        if let Some(fc) = file_config {
            TruncationConfig {
                max_lines: fc.max_lines.unwrap_or(args.max_lines),
                head_lines: fc.head_lines.unwrap_or(args.head_lines),
                tail_lines: fc.tail_lines.unwrap_or(args.tail_lines),
                max_line_length: fc.max_line_length.unwrap_or(args.max_line_length),
                head_chars: fc.head_chars.unwrap_or(args.head_chars),
                tail_chars: fc.tail_chars.unwrap_or(args.tail_chars),
            }
        } else {
            TruncationConfig::from(args)
        }
    }
}