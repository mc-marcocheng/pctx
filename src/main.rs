//! pctx - Generate LLM-ready context from your codebase
//!
//! This is the main entry point for the CLI application.

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

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
    if cli.global.no_color || !io::stderr().is_terminal() {
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
    let start_time = std::time::Instant::now();

    // Scan for files (either from paths or stdin)
    let scanner = Scanner::new(&config);
    let files = if args.stdin {
        let paths = read_paths_from_stdin()?;
        if paths.is_empty() {
            return handle_no_files_matched(args, global);
        }
        scanner.scan_paths(paths)?
    } else {
        scanner.scan()?
    };

    if files.is_empty() {
        return handle_no_files_matched(args, global);
    }

    // Process content
    let processor = ContentProcessor::new(&config);
    let mut entries: Vec<FileEntry> = Vec::new();
    let mut file_errors: Vec<FileError> = Vec::new();
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

    // Dry run - just show what would happen
    if args.dry_run {
        // Estimate tokens for dry run display
        let sample_content = entries
            .iter()
            .map(|e| e.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        stats.estimate_tokens(&sample_content, &args.token_model);
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
                    format: format!("{:?}", args.output.format).to_lowercase(),
                    files: file_infos,
                }),
                stats: (&stats).into(),
            })
        } else {
            JsonResponse::Partial(PartialResponse {
                data: ResponseData::Context(ContextOutput {
                    content: formatted.clone(),
                    format: format!("{:?}", args.output.format).to_lowercase(),
                    files: file_infos,
                }),
                stats: (&stats).into(),
                errors: file_errors.clone(),
            })
        };

        // JSON goes to stdout
        println!("{}", serde_json::to_string_pretty(&response)?);

        // Write to file/clipboard as side effect (if requested)
        if args.output.output.is_some() || args.output.clipboard {
            write_output(&formatted, &args.output, global)?;
        }

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

fn handle_no_files_matched(
    args: &pctx::cli::GenerateArgs,
    global: &pctx::cli::GlobalArgs,
) -> Result<i32, PctxError> {
    if global.json {
        let response = JsonResponse::Error(ErrorResponse {
            code: error_codes::NO_FILES_MATCHED.to_string(),
            message: "No files matched the specified filters".to_string(),
            input: Some(serde_json::json!({
                "paths": args.paths,
                "exclude": args.filter.exclude,
                "include": args.filter.include,
                "stdin": args.stdin,
            })),
            suggestion: Some("Try broadening your filters or checking the paths exist".to_string()),
            transient: false,
            exit_code: exit::NO_MATCH,
        });
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else if !global.quiet {
        eprintln!("No files matched the specified filters.");
        eprintln!("Hint: use --no-default-excludes to include commonly excluded directories");
    }
    Ok(exit::NO_MATCH)
}

fn write_output(
    content: &str,
    args: &pctx::cli::OutputArgs,
    global: &pctx::cli::GlobalArgs,
) -> Result<(), PctxError> {
    if let Some(ref path) = args.output {
        file::write(path, content, args.force)?;
        if !global.json && !global.quiet {
            eprintln!("Written to: {}", path.display());
        }
    } else if args.clipboard {
        clipboard::write(content)?;
        if !global.json && !global.quiet {
            eprintln!("✓ Copied to clipboard ({} bytes)", content.len());
        }
    } else {
        // Stdout
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
            let files = scanner.scan()?;

            // Check if output should be suppressed (either local --quiet or global --quiet)
            let suppress_extra = *quiet || global.quiet;

            if suppress_extra {
                // Bare output - one path per line, perfect for piping
                for file in &files {
                    println!("{}", file.display());
                }
            } else if global.json {
                let file_infos: Vec<FileInfo> = files
                    .iter()
                    .filter_map(|f| FileInfo::try_from_path(f).ok())
                    .collect();

                let response = JsonResponse::Success(SuccessResponse {
                    data: ResponseData::FileList(file_infos),
                    stats: StatsJson::new(files.len()),
                });
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                for file in &files {
                    println!("{}", file.display());
                }
                eprintln!("\n{} files", files.len());
            }

            Ok(if files.is_empty() {
                exit::NO_MATCH
            } else {
                exit::SUCCESS
            })
        }
        FilesCommands::Tree { filter } => {
            let config = Config::from_filter_args(filter, global)?;
            let scanner = Scanner::new(&config);
            let files = scanner.scan()?;
            let tree_struct = tree::build_tree(&files);

            if global.json {
                let response = JsonResponse::Success(SuccessResponse {
                    data: ResponseData::Tree(TreeOutput {
                        tree: tree::tree_to_string(&tree_struct),
                    }),
                    stats: StatsJson::new(files.len()),
                });
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                tree::print_tree(&tree_struct);
                if !global.quiet {
                    eprintln!("\n{} files", files.len());
                }
            }

            Ok(exit::SUCCESS)
        }
    }
}

fn run_config_command(
    cmd: &ConfigCommands,
    global: &pctx::cli::GlobalArgs,
) -> Result<i32, PctxError> {
    match cmd {
        ConfigCommands::Show => {
            let config = pctx::config::file::find_and_load()?;
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
