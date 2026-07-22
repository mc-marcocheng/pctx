//! Configuration handling for pctx.
//!
//! Configuration is built from multiple sources in order of precedence:
//! 1. Command-line arguments (highest)
//! 2. Config file (.pctx.toml)
//! 3. Built-in defaults (lowest)

pub mod defaults;
pub mod file;

use std::collections::HashSet;
use std::path::PathBuf;

use crate::cli::{ContentFormat, FilterArgs, GenerateArgs, GlobalArgs, TruncationArgs};
use crate::error::PctxError;

/// A resolved path alias mapping a display name to a canonicalized root directory
#[derive(Debug, Clone)]
pub struct PathAlias {
    pub alias: String,
    pub root: PathBuf,
}

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

/// Complete resolved configuration for a pctx operation
#[derive(Debug, Clone)]
pub struct Config {
    pub paths: Vec<PathBuf>,
    pub path_aliases: Vec<PathAlias>,
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
        let file_config = Self::load_file_config(global)?;

        let (exclude_patterns, include_patterns) =
            Self::build_patterns(&args.filter, file_config.as_ref());

        // Merge truncation settings: CLI args override file config
        let truncation = Self::build_truncation(&args.truncation, file_config.as_ref());

        let path_aliases = Self::build_path_aliases(&args.path_aliases)?;

        Ok(Self {
            paths: if args.paths.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                args.paths.clone()
            },
            path_aliases,
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
        let file_config = Self::load_file_config(global)?;

        let (exclude_patterns, include_patterns) =
            Self::build_patterns(filter, file_config.as_ref());

        Ok(Self {
            paths: vec![PathBuf::from(".")],
            path_aliases: Vec::new(),
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

    /// Load file configuration with proper error handling
    ///
    /// - `--no-config`: don't load anything.
    /// - `--config FILE`: load exactly that file and propagate errors.
    /// - No option: search current directory and parents; a malformed
    ///   auto-discovered config warns and continues rather than failing.
    fn load_file_config(global: &GlobalArgs) -> Result<Option<file::FileConfig>, PctxError> {
        if global.no_config {
            return Ok(None);
        }

        if let Some(path) = global.config.as_deref() {
            return file::load_config(path).map(Some);
        }

        let Some(path) = file::find_config_file() else {
            return Ok(None);
        };

        match file::load_config(&path) {
            Ok(config) => Ok(Some(config)),
            Err(error) => {
                if !global.quiet {
                    eprintln!(
                        "Warning: failed to load config file {}: {}",
                        path.display(),
                        error
                    );
                }
                Ok(None)
            }
        }
    }

    /// Parse and validate `--path-alias ALIAS=PATH` values.
    ///
    /// Nested roots are sorted so the most specific root wins when
    /// resolving a path against multiple aliases.
    fn build_path_aliases(values: &[String]) -> Result<Vec<PathAlias>, PctxError> {
        let mut aliases = Vec::new();
        let mut seen_aliases = HashSet::new();
        let mut seen_roots = HashSet::new();

        for value in values {
            let Some((alias, path)) = value.split_once('=') else {
                return Err(PctxError::ConfigError(format!(
                    "invalid path alias '{}'; expected ALIAS=PATH",
                    value
                )));
            };

            let alias = alias.trim();
            let path = path.trim();

            if alias.is_empty() || path.is_empty() {
                return Err(PctxError::ConfigError(format!(
                    "invalid path alias '{}'; alias and path must not be empty",
                    value
                )));
            }

            if alias == "."
                || alias == ".."
                || alias.contains('/')
                || alias.contains('\\')
                || !alias
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            {
                return Err(PctxError::ConfigError(format!(
                    "invalid alias '{}'; use letters, numbers, '.', '-' or '_'",
                    alias
                )));
            }

            if !seen_aliases.insert(alias.to_string()) {
                return Err(PctxError::ConfigError(format!(
                    "duplicate path alias '{}'",
                    alias
                )));
            }

            let input_root = PathBuf::from(path);
            let root = dunce::canonicalize(&input_root)
                .map_err(|_| PctxError::DirectoryNotFound(input_root.clone()))?;

            if !root.is_dir() {
                return Err(PctxError::DirectoryNotFound(root));
            }

            if !seen_roots.insert(root.clone()) {
                return Err(PctxError::ConfigError(format!(
                    "multiple aliases refer to the same root: {}",
                    root.display()
                )));
            }

            aliases.push(PathAlias {
                alias: alias.to_string(),
                root,
            });
        }

        // Nested roots must win over their parents.
        aliases.sort_by(|a, b| {
            b.root
                .components()
                .count()
                .cmp(&a.root.components().count())
        });

        Ok(aliases)
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
    /// CLI args (Some) take precedence over file config, which takes precedence over defaults
    fn build_truncation(
        args: &TruncationArgs,
        file_config: Option<&file::FileConfig>,
    ) -> TruncationConfig {
        let defaults = TruncationConfig::default();

        // If --no-truncation is set, ignore everything else
        if args.no_truncation {
            return TruncationConfig {
                max_lines: 0,
                max_line_length: 0,
                ..defaults
            };
        }

        let fc = file_config;

        TruncationConfig {
            max_lines: args
                .max_lines
                .or(fc.and_then(|f| f.max_lines))
                .unwrap_or(defaults.max_lines),
            head_lines: args
                .head_lines
                .or(fc.and_then(|f| f.head_lines))
                .unwrap_or(defaults.head_lines),
            tail_lines: args
                .tail_lines
                .or(fc.and_then(|f| f.tail_lines))
                .unwrap_or(defaults.tail_lines),
            max_line_length: args
                .max_line_length
                .or(fc.and_then(|f| f.max_line_length))
                .unwrap_or(defaults.max_line_length),
            head_chars: args
                .head_chars
                .or(fc.and_then(|f| f.head_chars))
                .unwrap_or(defaults.head_chars),
            tail_chars: args
                .tail_chars
                .or(fc.and_then(|f| f.tail_chars))
                .unwrap_or(defaults.tail_chars),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::file::FileConfig;

    fn base_filter() -> FilterArgs {
        FilterArgs {
            exclude: Vec::new(),
            include: Vec::new(),
            hidden: false,
            no_default_excludes: false,
            no_gitignore: false,
            max_size: 1024,
            max_depth: 0,
        }
    }

    fn base_trunc() -> TruncationArgs {
        TruncationArgs {
            no_truncation: false,
            max_lines: None,
            head_lines: None,
            tail_lines: None,
            max_line_length: None,
            head_chars: None,
            tail_chars: None,
        }
    }

    #[test]
    fn build_patterns_precedence_defaults_file_cli() {
        let mut filter = base_filter();
        filter.exclude = vec!["cli-exclude".into()];
        filter.include = vec!["cli-include".into()];
        let file_cfg = FileConfig {
            exclude: vec!["file-exclude".into()],
            include: vec!["file-include".into()],
            ..Default::default()
        };

        let (excl, incl) = Config::build_patterns(&filter, Some(&file_cfg));

        // Defaults come first
        assert_eq!(excl[0], defaults::DEFAULT_EXCLUDES[0]);
        // Then file, then CLI
        let defaults_len = defaults::DEFAULT_EXCLUDES.len();
        assert_eq!(excl[defaults_len], "file-exclude");
        assert_eq!(excl[defaults_len + 1], "cli-exclude");

        // Include: file first, then CLI
        assert_eq!(incl, vec!["file-include", "cli-include"]);
    }

    #[test]
    fn build_patterns_no_default_excludes_zeros_defaults() {
        let mut filter = base_filter();
        filter.no_default_excludes = true;
        filter.exclude = vec!["only".into()];

        let (excl, _) = Config::build_patterns(&filter, None);
        assert_eq!(excl, vec!["only"]);
    }

    #[test]
    fn build_patterns_without_file_config() {
        let filter = base_filter();
        let (excl, incl) = Config::build_patterns(&filter, None);

        assert_eq!(excl.len(), defaults::DEFAULT_EXCLUDES.len());
        assert!(incl.is_empty());
    }

    #[test]
    fn build_truncation_no_truncation_short_circuits() {
        let mut trunc = base_trunc();
        trunc.no_truncation = true;
        let file_cfg = FileConfig {
            max_lines: Some(999),
            head_lines: Some(42),
            ..Default::default()
        };

        let result = Config::build_truncation(&trunc, Some(&file_cfg));

        // max_lines and max_line_length are zeroed
        assert_eq!(result.max_lines, 0);
        assert_eq!(result.max_line_length, 0);
        // Other defaults are preserved (file_cfg is ignored)
        let defaults = TruncationConfig::default();
        assert_eq!(result.head_lines, defaults.head_lines);
        assert_eq!(result.tail_lines, defaults.tail_lines);
        assert_eq!(result.head_chars, defaults.head_chars);
        assert_eq!(result.tail_chars, defaults.tail_chars);
    }

    #[test]
    fn build_truncation_cli_overrides_file_overrides_default() {
        let mut trunc = base_trunc();
        trunc.max_lines = Some(100); // CLI overrides file
                                     // head_lines not set on CLI — should fall to file
        let file_cfg = FileConfig {
            max_lines: Some(999),
            head_lines: Some(7),
            ..Default::default()
        };

        let result = Config::build_truncation(&trunc, Some(&file_cfg));

        assert_eq!(result.max_lines, 100); // CLI wins
        assert_eq!(result.head_lines, 7); // file wins
        assert_eq!(result.tail_lines, TruncationConfig::default().tail_lines); // default wins
    }

    #[test]
    fn build_truncation_no_file_config_uses_cli_or_default() {
        let mut trunc = base_trunc();
        trunc.max_line_length = Some(80);

        let result = Config::build_truncation(&trunc, None);

        assert_eq!(result.max_line_length, 80);
        assert_eq!(result.max_lines, TruncationConfig::default().max_lines);
    }

    #[test]
    fn truncation_config_defaults_are_nonzero() {
        // Sanity: the defaults the builder falls back to aren't accidentally zero.
        let d = TruncationConfig::default();
        assert!(d.max_lines > 0);
        assert!(d.max_line_length > 0);
        assert!(d.head_lines > 0);
        assert!(d.tail_lines > 0);
    }

    #[test]
    fn build_path_aliases_unrelated_roots() {
        let dir_a = tempfile::TempDir::new().unwrap();
        let dir_b = tempfile::TempDir::new().unwrap();

        let values = vec![
            format!("api={}", dir_a.path().display()),
            format!("shared={}", dir_b.path().display()),
        ];

        let aliases = Config::build_path_aliases(&values).unwrap();
        assert_eq!(aliases.len(), 2);
        assert!(aliases.iter().any(|a| a.alias == "api"));
        assert!(aliases.iter().any(|a| a.alias == "shared"));
    }

    #[test]
    fn build_path_aliases_nested_most_specific_first() {
        let dir = tempfile::TempDir::new().unwrap();
        let nested = dir.path().join("nested");
        std::fs::create_dir(&nested).unwrap();

        let values = vec![
            format!("outer={}", dir.path().display()),
            format!("inner={}", nested.display()),
        ];

        let aliases = Config::build_path_aliases(&values).unwrap();
        assert_eq!(aliases[0].alias, "inner");
        assert_eq!(aliases[1].alias, "outer");
    }

    #[test]
    fn build_path_aliases_duplicate_alias_fails() {
        let dir = tempfile::TempDir::new().unwrap();
        let values = vec![
            format!("dup={}", dir.path().display()),
            format!("dup={}", dir.path().display()),
        ];

        assert!(Config::build_path_aliases(&values).is_err());
    }

    #[test]
    fn build_path_aliases_duplicate_root_fails() {
        let dir = tempfile::TempDir::new().unwrap();
        let values = vec![
            format!("a={}", dir.path().display()),
            format!("b={}", dir.path().display()),
        ];

        assert!(Config::build_path_aliases(&values).is_err());
    }

    #[test]
    fn build_path_aliases_invalid_format_fails() {
        let values = vec!["not-a-valid-alias".to_string()];
        assert!(Config::build_path_aliases(&values).is_err());
    }

    #[test]
    fn build_path_aliases_nonexistent_root_fails() {
        let values = vec!["x=/definitely/does/not/exist/anywhere".to_string()];
        assert!(Config::build_path_aliases(&values).is_err());
    }

    #[test]
    fn build_path_aliases_empty_is_ok() {
        assert!(Config::build_path_aliases(&[]).unwrap().is_empty());
    }
}
