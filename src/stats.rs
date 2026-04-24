//! Statistics collection and reporting.

use colored::*;

use crate::content::FileEntry;

/// Statistics about processed files
#[derive(Debug, Default, Clone)]
pub struct Stats {
    pub file_count: usize,
    pub total_lines: usize,
    pub total_bytes: usize,
    pub truncated_count: usize,
    pub skipped_count: usize,
    pub token_estimate: Option<usize>,
    pub duration_ms: u64,
}

impl Stats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file entry to stats
    pub fn add_file(&mut self, entry: &FileEntry) {
        self.file_count += 1;
        self.total_lines += entry.original_lines;
        self.total_bytes += entry.original_bytes;

        if entry.truncated {
            self.truncated_count += 1;
        }
    }

    /// Estimate token count for the content
    #[cfg(feature = "tokens")]
    pub fn estimate_tokens(&mut self, content: &str, model: &str) {
        use tiktoken_rs::get_bpe_from_model;

        // Map common model names
        let model_name = match model.to_lowercase().as_str() {
            "gpt-4" | "gpt4" | "gpt-4o" | "gpt-4-turbo" => "gpt-4",
            "gpt-3.5" | "gpt-3.5-turbo" | "gpt35" => "gpt-3.5-turbo",
            "claude" | "claude-3" | "claude-3.5" => "gpt-4", // Use GPT-4 tokenizer as approximation
            _ => "gpt-4",
        };

        if let Ok(bpe) = get_bpe_from_model(model_name) {
            self.token_estimate = Some(bpe.encode_ordinary(content).len());
        } else {
            // Fallback to rough estimate
            self.token_estimate = Some(content.len() / 4);
        }
    }

    /// Estimate token count without tiktoken
    #[cfg(not(feature = "tokens"))]
    pub fn estimate_tokens(&mut self, content: &str, _model: &str) {
        // Rough estimate: ~4 characters per token for English text/code
        self.token_estimate = Some(content.len() / 4);
    }

    /// Print stats summary to stderr
    pub fn print_summary(&self) {
        eprintln!();
        eprintln!("{}", "─".repeat(40).dimmed());
        eprintln!("{}", "Statistics".bold());
        eprintln!("{}", "─".repeat(40).dimmed());
        eprintln!(
            "  {}      {}",
            "Files:".dimmed(),
            self.file_count.to_string().cyan()
        );
        eprintln!(
            "  {}      {}",
            "Lines:".dimmed(),
            format_number(self.total_lines).cyan()
        );
        eprintln!(
            "  {}       {}",
            "Size:".dimmed(),
            format_bytes(self.total_bytes).cyan()
        );

        if self.truncated_count > 0 {
            eprintln!(
                "  {}  {}",
                "Truncated:".dimmed(),
                self.truncated_count.to_string().yellow()
            );
        }

        if let Some(tokens) = self.token_estimate {
            eprintln!(
                "  {}     ~{}",
                "Tokens:".dimmed(),
                format_number(tokens).cyan()
            );
        }

        if self.duration_ms > 0 {
            eprintln!(
                "  {}   {}ms",
                "Duration:".dimmed(),
                self.duration_ms.to_string().dimmed()
            );
        }

        eprintln!("{}", "─".repeat(40).dimmed());
    }
}

/// Format a number with thousand separators
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }
    result
}

/// Format bytes as human-readable size
fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_entry(lines: usize, bytes: usize, truncated: bool) -> FileEntry {
        FileEntry {
            absolute_path: PathBuf::from("/tmp/x"),
            relative_path: "x".into(),
            extension: String::new(),
            original_bytes: bytes,
            original_lines: lines,
            line_count: lines,
            truncated,
            truncated_lines: if truncated { 5 } else { 0 },
            content: String::new(),
        }
    }

    #[test]
    fn format_number_inserts_thousand_separators() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1_000), "1,000");
        assert_eq!(format_number(12_345), "12,345");
        assert_eq!(format_number(1_000_000), "1,000,000");
        assert_eq!(format_number(1_234_567_890), "1,234,567,890");
    }

    #[test]
    fn format_bytes_picks_correct_unit() {
        assert_eq!(format_bytes(0), "0 bytes");
        assert_eq!(format_bytes(1023), "1023 bytes");
        // KB boundary
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        // MB boundary
        assert_eq!(format_bytes(1024 * 1024 - 1), "1024.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        // GB boundary
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn add_file_accumulates_counters() {
        let mut stats = Stats::new();
        stats.add_file(&make_entry(10, 100, false));
        stats.add_file(&make_entry(20, 200, true));
        stats.add_file(&make_entry(5, 50, false));

        assert_eq!(stats.file_count, 3);
        assert_eq!(stats.total_lines, 35);
        assert_eq!(stats.total_bytes, 350);
        assert_eq!(stats.truncated_count, 1);
    }

    #[test]
    fn add_file_truncated_flag_increments_only_on_true() {
        let mut stats = Stats::new();
        for _ in 0..5 {
            stats.add_file(&make_entry(1, 1, false));
        }
        assert_eq!(stats.truncated_count, 0);

        for _ in 0..3 {
            stats.add_file(&make_entry(1, 1, true));
        }
        assert_eq!(stats.truncated_count, 3);
    }

    #[test]
    fn estimate_tokens_produces_estimate() {
        let mut stats = Stats::new();
        stats.estimate_tokens("hello world, this is some content", "gpt-4");
        // Must populate the field; exact count varies between fallback (len/4)
        // and the tiktoken-backed implementation, so just assert presence/sanity.
        let estimate = stats.token_estimate.expect("token estimate populated");
        assert!(estimate > 0);
    }

    #[test]
    fn estimate_tokens_on_empty_content() {
        let mut stats = Stats::new();
        stats.estimate_tokens("", "gpt-4");
        assert_eq!(stats.token_estimate, Some(0));
    }

    #[test]
    fn new_starts_at_zero() {
        let stats = Stats::new();
        assert_eq!(stats.file_count, 0);
        assert_eq!(stats.total_lines, 0);
        assert_eq!(stats.total_bytes, 0);
        assert_eq!(stats.truncated_count, 0);
        assert!(stats.token_estimate.is_none());
    }
}
