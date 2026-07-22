//! pctx - Generate LLM-ready context from your codebase
//!
//! This is the main entry point for the CLI application.

use std::fs::File;
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use serde::Serialize;

use pctx::cli::{Cli, Commands, ConfigCommands, FilesCommands};
use pctx::config::Config;
use pctx::content::{ContentProcessor, FileEntry};
use pctx::error::PctxError;
use pctx::exit_codes::exit;
use pctx::output::json_types::{
    error_codes, ContextOutput, ErrorResponse, FileError, FileInfo, JsonResponse, PartialResponse,
    ResponseData, StatsJson, SuccessResponse, TreeOutput,
};
use pctx::output::{clipboard, file, formatter, tree};
use pctx::scanner::Scanner;
use pctx::stats::Stats;

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Disable colors if requested or not a terminal
    if cli.global.no_color
        || std::env::var_os("NO_COLOR").is_some()
        || (!io::stderr().is_terminal() && !io::stdout().is_terminal())
    {
        colored::control::set_override(false);
    }

    // Run the appropriate command
    let result = run(&cli);

    // Handle the result
    match result {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            let (response, code) = error_to_response(&e);

            if cli.global.json {
                // JSON error to stdout (this is the API contract)
                if let Ok(json) = serde_json::to_string_pretty(&response) {
                    println!("{}", json);
                }
            } else {
                // Human-readable error to stderr
                eprintln!("error: {}", e);
                if let Some(suggestion) = e.suggestion() {
                    eprintln!("suggestion: {}", suggestion);
                }
            }

            ExitCode::from(code as u8)
        }
    }
}

fn run(cli: &Cli) -> Result<i32, PctxError> {
    match &cli.command {
        Some(Commands::Files(files_cmd)) => run_files_command(files_cmd, &cli.global),
        Some(Commands::Config(config_cmd)) => run_config_command(config_cmd, &cli.global),
        Some(Commands::Capabilities) => run_capabilities(&cli.global),
        Some(Commands::Completions { shell }) => {
            generate_completions(shell);
            Ok(exit::SUCCESS)
        }
        None => run_generate_command(&cli.generate, &cli.global),
    }
}

fn run_generate_command(
    args: &pctx::cli::GenerateArgs,
    global: &pctx::cli::GlobalArgs,
) -> Result<i32, PctxError> {
    let config = Config::from_args(args, global)?;

    let uses_external_path_input = args.stdin || args.stdin0 || args.paths_file0.is_some();

    if uses_external_path_input && !args.paths.is_empty() && !global.quiet {
        eprintln!("Warning: positional paths are ignored when an external path input mode is used");
    }

    let start_time = std::time::Instant::now();

    // Scan for files (either from paths or an external path input mode)
    let scanner = Scanner::new(&config);
    let scan_result = if args.stdin {
        let paths = read_paths_from_stdin()?;
        if paths.is_empty() {
            return handle_no_files_matched(args, global, &config);
        }
        scanner.scan_paths(paths)?
    } else if args.stdin0 {
        let paths = read_paths_from_stdin0()?;
        if paths.is_empty() {
            return handle_no_files_matched(args, global, &config);
        }
        scanner.scan_paths(paths)?
    } else if let Some(path) = args.paths_file0.as_deref() {
        let paths = read_paths_from_file0(path)?;
        if paths.is_empty() {
            return handle_no_files_matched(args, global, &config);
        }
        scanner.scan_paths(paths)?
    } else {
        scanner.scan()?
    };

    if scan_result.files.is_empty() {
        if let Some((_, err)) = scan_result.errors.into_iter().next() {
            return Err(err);
        }
        return handle_no_files_matched(args, global, &config);
    }

    let files = scan_result.files;

    // Convert scan errors into file errors for reporting
    for (path, err) in &scan_result.errors {
        if global.verbose && !global.json {
            eprintln!("Warning: {}: {}", path.display(), err);
        }
    }
    let mut scan_file_errors: Vec<FileError> = scan_result
        .errors
        .iter()
        .map(|(path, err)| FileError {
            path: path.to_string_lossy().to_string(),
            code: err.code().to_string(),
            message: err.to_string(),
            transient: err.is_transient(),
        })
        .collect();

    // Process content
    let processor = ContentProcessor::new(&config);
    let mut entries: Vec<FileEntry> = Vec::new();
    let mut file_errors: Vec<FileError> = Vec::new();
    file_errors.append(&mut scan_file_errors);
    let mut stats = Stats::new();

    for file_path in files {
        match processor.process(&file_path) {
            Ok(entry) => {
                stats.add_file(&entry);
                entries.push(entry);
            }
            Err(e) => {
                if global.verbose && !global.json {
                    eprintln!("Skipped {}: {}", file_path.display(), e);
                }
                file_errors.push(FileError {
                    path: file_path.to_string_lossy().to_string(),
                    code: e.code().to_string(),
                    message: e.to_string(),
                    transient: e.is_transient(),
                });
            }
        }
    }

    stats.skipped_count = file_errors.len();

    // Dry run - just show what would happen
    if args.dry_run {
        let formatted = formatter::format_output(&entries, &config)?;
        stats.estimate_tokens(&formatted, &args.token_model);
        return handle_dry_run(
            &entries,
            &file_errors,
            &stats,
            global,
            config.absolute_paths,
        );
    }

    // Format output
    let formatted = formatter::format_output(&entries, &config)?;
    stats.duration_ms = start_time.elapsed().as_millis() as u64;

    // Estimate tokens
    if args.output.stats || global.json {
        stats.estimate_tokens(&formatted, &args.token_model);
    }

    // Handle JSON output
    if global.json {
        let file_infos: Vec<FileInfo> = entries
            .iter()
            .map(|e| FileInfo::from_entry(e, config.absolute_paths))
            .collect();

        let response = if file_errors.is_empty() {
            JsonResponse::Success(SuccessResponse {
                data: ResponseData::Context(ContextOutput {
                    content: formatted.clone(),
                    format: args.output.format.as_str().to_string(),
                    files: file_infos,
                }),
                stats: (&stats).into(),
            })
        } else {
            JsonResponse::Partial(PartialResponse {
                data: ResponseData::Context(ContextOutput {
                    content: formatted.clone(),
                    format: args.output.format.as_str().to_string(),
                    files: file_infos,
                }),
                stats: (&stats).into(),
                errors: file_errors.clone(),
            })
        };

        // Complete requested side effects first, so a failure (e.g. clipboard
        // access) produces a single JSON error response instead of a success
        // response followed by an error.
        if args.output.output.is_some() || args.output.clipboard {
            write_output(&formatted, &args.output, global)?;
        }

        // Only publish success/partial JSON after side effects succeeded.
        println!("{}", serde_json::to_string_pretty(&response)?);

        return Ok(if file_errors.is_empty() {
            exit::SUCCESS
        } else {
            exit::PARTIAL
        });
    }

    // Non-JSON output
    write_output(&formatted, &args.output, global)?;

    // Stats to stderr
    if args.output.stats {
        stats.print_summary();
    }

    Ok(if file_errors.is_empty() {
        exit::SUCCESS
    } else {
        exit::PARTIAL
    })
}

/// Read file paths from stdin, one per line
fn read_paths_from_stdin() -> Result<Vec<PathBuf>, PctxError> {
    let stdin = io::stdin();
    let mut paths = Vec::new();

    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            paths.push(PathBuf::from(trimmed));
        }
    }

    Ok(paths)
}

/// Split NUL-delimited bytes into paths, decoding non-UTF-8 bytes lossily.
fn read_nul_delimited_paths<R: Read>(mut reader: R) -> Result<Vec<PathBuf>, PctxError> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;

    let paths = bytes
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(|part| PathBuf::from(String::from_utf8_lossy(part).into_owned()))
        .collect();

    Ok(paths)
}

/// Read NUL-delimited file paths from stdin
fn read_paths_from_stdin0() -> Result<Vec<PathBuf>, PctxError> {
    let stdin = io::stdin();
    read_nul_delimited_paths(stdin.lock())
}

/// Read NUL-delimited file paths from a file
fn read_paths_from_file0(path: &Path) -> Result<Vec<PathBuf>, PctxError> {
    let file = File::open(path).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => PctxError::FileNotFound(path.to_path_buf()),
        io::ErrorKind::PermissionDenied => PctxError::PermissionDenied(path.to_path_buf()),
        _ => PctxError::Io(error),
    })?;

    read_nul_delimited_paths(file)
}

/// Return true when a path contains a dot-prefixed normal component.
///
/// Examples:
/// - `.github`
/// - `./.github/workflows`
/// - `project/.config/file.toml`
///
/// `.` and `..` are represented as separate Component variants and therefore
/// are not treated as hidden names.
fn has_hidden_component(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(name) => name.to_string_lossy().starts_with('.'),
        _ => false,
    })
}

/// Build contextual suggestions for an empty scan result.
///
/// Filtering layers are independent:
/// - `--hidden` controls dot-prefixed paths
/// - `--no-default-excludes` controls built-in patterns
/// - `--no-gitignore` controls gitignore rules
fn no_match_hints(args: &pctx::cli::GenerateArgs, config: &Config) -> Vec<String> {
    // An explicitly requested hidden path is the strongest available signal.
    if !config.include_hidden {
        if let Some(path) = args.paths.iter().find(|path| has_hidden_component(path)) {
            return vec![format!(
                "`{}` contains a dot-prefixed path component and is hidden by default. \
                 Add `--hidden`. `--no-default-excludes` only disables built-in \
                 exclusion patterns.",
                path.display()
            )];
        }
    }

    let mut hints = Vec::new();

    if !config.include_patterns.is_empty() {
        hints.push(
            "Include patterns are active; check whether the files match `--include` \
             or the `include` entries in `.pctx.toml`."
                .to_string(),
        );
    }

    if !args.filter.exclude.is_empty() {
        hints.push(
            "Custom exclude patterns are active; check the supplied `--exclude` patterns."
                .to_string(),
        );
    }

    if !config.include_hidden {
        hints.push(
            "Dot-prefixed files and directories are hidden by default; add `--hidden` \
             if they should be included."
                .to_string(),
        );
    }

    if config.use_default_excludes {
        hints.push(
            "Built-in exclusions are active; add `--no-default-excludes` to disable them."
                .to_string(),
        );
    }

    if config.use_gitignore {
        hints.push(
            "Gitignore rules are active; add `--no-gitignore` to ignore those rules.".to_string(),
        );
    }

    if hints.is_empty() {
        hints.push(
            "Check that the selected paths contain readable, non-binary files within \
             the configured size and depth limits."
                .to_string(),
        );
    }

    hints
}

fn handle_no_files_matched(
    args: &pctx::cli::GenerateArgs,
    global: &pctx::cli::GlobalArgs,
    config: &Config,
) -> Result<i32, PctxError> {
    let hints = no_match_hints(args, config);
    let suggestion = hints.join(" ");

    if global.json {
        let response = JsonResponse::Error(ErrorResponse {
            code: error_codes::NO_FILES_MATCHED.to_string(),
            message: "No files matched the specified filters".to_string(),
            input: Some(serde_json::json!({
                "paths": args.paths,
                "exclude": args.filter.exclude,
                "include": args.filter.include,
                "hidden": args.filter.hidden,
                "no_default_excludes": args.filter.no_default_excludes,
                "no_gitignore": args.filter.no_gitignore,
                "max_size_kb": args.filter.max_size,
                "max_depth": args.filter.max_depth,
                "stdin": args.stdin,
                "stdin0": args.stdin0,
                "paths_file0": args.paths_file0,
            })),
            suggestion: Some(suggestion),
            transient: false,
            exit_code: exit::NO_MATCH,
        });

        println!("{}", serde_json::to_string_pretty(&response)?);
    } else if !global.quiet {
        eprintln!("No files matched the specified filters.");
        for hint in hints {
            eprintln!("Hint: {}", hint);
        }
    }

    Ok(exit::NO_MATCH)
}

fn write_output(
    content: &str,
    args: &pctx::cli::OutputArgs,
    global: &pctx::cli::GlobalArgs,
) -> Result<(), PctxError> {
    let mut wrote_to_dest = false;

    if let Some(ref path) = args.output {
        file::write(path, content, args.force)?;
        if !global.json && !global.quiet {
            eprintln!("Written to: {}", path.display());
        }
        wrote_to_dest = true;
    }

    if args.clipboard {
        clipboard::write(content)?;
        if !global.json && !global.quiet {
            eprintln!("✓ Copied to clipboard ({} bytes)", content.len());
        }
        wrote_to_dest = true;
    }

    if !wrote_to_dest {
        print!("{}", content);
        io::stdout().flush().map_err(PctxError::Io)?;
    }

    Ok(())
}

fn run_files_command(
    cmd: &FilesCommands,
    global: &pctx::cli::GlobalArgs,
) -> Result<i32, PctxError> {
    match cmd {
        FilesCommands::List { filter, quiet } => {
            let config = Config::from_filter_args(filter, global)?;
            let scanner = Scanner::new(&config);
            let scan_result = scanner.scan()?;
            let files = scan_result.files;
            let errors = scan_result.errors;
            let relative_files = relativize_paths(&files);
            let file_errors = scan_errors_to_file_errors(&errors);

            // Check if output should be suppressed (either local --quiet or global --quiet)
            let suppress_extra = *quiet || global.quiet;

            if suppress_extra {
                // Bare output - one path per line, perfect for piping
                for file in &relative_files {
                    println!("{}", file.display());
                }
            } else if global.json {
                let file_infos: Vec<FileInfo> = files
                    .iter()
                    .filter_map(|f| FileInfo::try_from_path(f).ok())
                    .collect();

                let mut stats = StatsJson::new(files.len());
                stats.skipped_count = file_errors.len();

                let data = ResponseData::FileList(file_infos);
                let response = if file_errors.is_empty() {
                    JsonResponse::Success(SuccessResponse { data, stats })
                } else {
                    JsonResponse::Partial(PartialResponse {
                        data,
                        stats,
                        errors: file_errors.clone(),
                    })
                };
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                for file in &relative_files {
                    println!("{}", file.display());
                }
                eprintln!("\n{} files", files.len());
            }

            Ok(scan_exit_code(!files.is_empty(), &errors, &file_errors))
        }
        FilesCommands::Tree { filter } => {
            let config = Config::from_filter_args(filter, global)?;
            let scanner = Scanner::new(&config);
            let scan_result = scanner.scan()?;
            let files = scan_result.files;
            let errors = scan_result.errors;
            let relative_files = relativize_paths(&files);
            let tree_struct = tree::build_tree(&relative_files);
            let file_errors = scan_errors_to_file_errors(&errors);

            if global.json {
                let mut stats = StatsJson::new(files.len());
                stats.skipped_count = file_errors.len();

                let data = ResponseData::Tree(TreeOutput {
                    tree: tree::tree_to_string(&tree_struct),
                });
                let response = if file_errors.is_empty() {
                    JsonResponse::Success(SuccessResponse { data, stats })
                } else {
                    JsonResponse::Partial(PartialResponse {
                        data,
                        stats,
                        errors: file_errors.clone(),
                    })
                };
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                tree::print_tree(&tree_struct);
                if !global.quiet {
                    eprintln!("\n{} files", files.len());
                }
            }

            Ok(scan_exit_code(!files.is_empty(), &errors, &file_errors))
        }
    }
}

/// Convert scanner errors into structured `FileError`s for JSON reporting
fn scan_errors_to_file_errors(errors: &[(PathBuf, PctxError)]) -> Vec<FileError> {
    errors
        .iter()
        .map(|(path, error)| FileError {
            path: path.to_string_lossy().to_string(),
            code: error.code().to_string(),
            message: error.to_string(),
            transient: error.is_transient(),
        })
        .collect()
}

/// Determine the exit code for a scan that may have partially failed:
/// - `SUCCESS` if files were found and there were no errors
/// - `PARTIAL` if files were found and some paths failed
/// - `NO_MATCH` if no files matched and there were no scan errors
/// - the first error's exit code if nothing succeeded and errors exist
fn scan_exit_code(
    found_files: bool,
    errors: &[(PathBuf, PctxError)],
    file_errors: &[FileError],
) -> i32 {
    if found_files {
        if file_errors.is_empty() {
            exit::SUCCESS
        } else {
            exit::PARTIAL
        }
    } else if let Some((_, first_error)) = errors.first() {
        first_error.exit_code()
    } else {
        exit::NO_MATCH
    }
}

/// Machine-readable description of features supported by this build
#[derive(Debug, Serialize)]
struct Capabilities {
    schema_version: u32,
    name: &'static str,
    version: &'static str,
    clipboard: bool,
    tokens: bool,
    json_output: bool,
    stdin: bool,
    stdin0: bool,
    paths_file0: bool,
    path_aliases: bool,
    formats: Vec<&'static str>,
}

fn run_capabilities(global: &pctx::cli::GlobalArgs) -> Result<i32, PctxError> {
    let capabilities = Capabilities {
        schema_version: 1,
        name: "pctx",
        version: env!("CARGO_PKG_VERSION"),
        clipboard: cfg!(feature = "clipboard"),
        tokens: cfg!(feature = "tokens"),
        json_output: true,
        stdin: true,
        stdin0: true,
        paths_file0: true,
        path_aliases: true,
        formats: vec!["markdown", "xml", "plain"],
    };

    if global.json {
        println!("{}", serde_json::to_string_pretty(&capabilities)?);
    } else {
        println!("pctx {}", capabilities.version);
        println!("clipboard: {}", capabilities.clipboard);
        println!("tokens: {}", capabilities.tokens);
        println!("stdin0: {}", capabilities.stdin0);
        println!("paths_file0: {}", capabilities.paths_file0);
        println!("path_aliases: {}", capabilities.path_aliases);
        println!("formats: {}", capabilities.formats.join(", "));
    }

    Ok(exit::SUCCESS)
}

/// Load the file config honoring `--config`/`--no-config` selection
fn load_selected_file_config(
    global: &pctx::cli::GlobalArgs,
) -> Result<pctx::config::file::FileConfig, PctxError> {
    if global.no_config {
        return Ok(pctx::config::file::FileConfig::default());
    }

    if let Some(path) = global.config.as_deref() {
        return pctx::config::file::load_config(path);
    }

    pctx::config::file::find_and_load()
}

fn run_config_command(
    cmd: &ConfigCommands,
    global: &pctx::cli::GlobalArgs,
) -> Result<i32, PctxError> {
    match cmd {
        ConfigCommands::Show => {
            let config = load_selected_file_config(global)?;
            if global.json {
                println!("{}", serde_json::to_string_pretty(&config)?);
            } else {
                println!("{}", toml::to_string_pretty(&config)?);
            }
            Ok(exit::SUCCESS)
        }
        ConfigCommands::Init { force } => {
            let path = std::path::Path::new(".pctx.toml");
            pctx::config::file::write_template(path, *force)?;
            if global.json {
                println!(r#"{{"status": "success", "path": ".pctx.toml"}}"#);
            } else {
                eprintln!("Created .pctx.toml");
            }
            Ok(exit::SUCCESS)
        }
        ConfigCommands::Defaults => {
            let defaults = pctx::config::defaults::DEFAULT_EXCLUDES;
            if global.json {
                println!("{}", serde_json::to_string_pretty(&defaults)?);
            } else {
                for pattern in defaults {
                    println!("{}", pattern);
                }
            }
            Ok(exit::SUCCESS)
        }
    }
}

fn error_to_response(e: &PctxError) -> (JsonResponse, i32) {
    let exit_code = e.exit_code();
    let response = JsonResponse::Error(ErrorResponse {
        code: e.code().to_string(),
        message: e.to_string(),
        input: e.input_context(),
        suggestion: e.suggestion().map(String::from),
        transient: e.is_transient(),
        exit_code,
    });
    (response, exit_code)
}

fn generate_completions(shell: &pctx::cli::Shell) {
    use clap::CommandFactory;
    use clap_complete::generate;

    let mut cmd = Cli::command();
    let shell_type = match shell {
        pctx::cli::Shell::Bash => clap_complete::Shell::Bash,
        pctx::cli::Shell::Zsh => clap_complete::Shell::Zsh,
        pctx::cli::Shell::Fish => clap_complete::Shell::Fish,
        pctx::cli::Shell::PowerShell => clap_complete::Shell::PowerShell,
        pctx::cli::Shell::Elvish => clap_complete::Shell::Elvish,
    };
    generate(shell_type, &mut cmd, "pctx", &mut io::stdout());
}

fn relativize_paths(files: &[PathBuf]) -> Vec<PathBuf> {
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| dunce::canonicalize(&p).ok())
        .unwrap_or_else(|| PathBuf::from("."));
    files
        .iter()
        .map(|f| f.strip_prefix(&cwd).unwrap_or(f).to_path_buf())
        .collect()
}

fn handle_dry_run(
    entries: &[FileEntry],
    errors: &[FileError],
    stats: &Stats,
    global: &pctx::cli::GlobalArgs,
    absolute_paths: bool,
) -> Result<i32, PctxError> {
    if global.json {
        let file_infos: Vec<FileInfo> = entries
            .iter()
            .map(|e| FileInfo::from_entry(e, absolute_paths))
            .collect();

        let response = if errors.is_empty() {
            JsonResponse::Success(SuccessResponse {
                data: ResponseData::FileList(file_infos),
                stats: stats.into(),
            })
        } else {
            JsonResponse::Partial(PartialResponse {
                data: ResponseData::FileList(file_infos),
                stats: stats.into(),
                errors: errors.to_vec(),
            })
        };

        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        use colored::*;

        eprintln!("{}", "Dry run - files that would be included:".yellow());
        eprintln!();

        for entry in entries {
            let display_path = entry.display_path(absolute_paths);
            let marker = if entry.truncated {
                " [truncated]".dimmed().to_string()
            } else {
                String::new()
            };
            eprintln!(
                "  {} ({} lines){}",
                display_path.green(),
                entry.original_lines,
                marker
            );
        }

        if !errors.is_empty() {
            eprintln!();
            eprintln!("{}", "Skipped files:".yellow());
            for err in errors {
                eprintln!("  {} ({})", err.path.red(), err.code);
            }
        }

        eprintln!();
        eprintln!(
            "Total: {} files, ~{} tokens",
            entries.len(),
            stats.token_estimate.unwrap_or(0)
        );
    }

    Ok(exit::SUCCESS)
}
