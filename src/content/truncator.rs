//! Content truncation for long files and lines.

use crate::config::TruncationConfig;

/// Marker text used in truncation - used to identify our own markers
const LINES_OMITTED_MARKER: &str = "lines omitted";

/// Truncate content based on configuration
///
/// Returns: (truncated_content, was_truncated, lines_removed)
pub fn truncate_content(content: &str, config: &TruncationConfig) -> (String, bool, usize) {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    let mut truncated = false;
    let mut truncated_lines = 0;

    // Check if we need line truncation
    let processed_lines: Vec<String> = if config.max_lines > 0 && total_lines > config.max_lines {
        truncated = true;
        let head_count = config.head_lines.min(total_lines);
        let tail_count = config
            .tail_lines
            .min(total_lines.saturating_sub(head_count));
        let tail_start = total_lines.saturating_sub(tail_count);

        truncated_lines = total_lines.saturating_sub(head_count + tail_count);

        let head = &lines[..head_count];
        let tail = &lines[tail_start..];

        let mut result: Vec<String> = head.iter().map(|s| s.to_string()).collect();

        if truncated_lines > 0 {
            // Single line marker without extra newlines (join handles spacing)
            result.push(format!(
                "... [{} {}] ...",
                truncated_lines, LINES_OMITTED_MARKER
            ));
        }

        result.extend(tail.iter().map(|s| s.to_string()));
        result
    } else {
        lines.iter().map(|s| s.to_string()).collect()
    };

    // Process each line for length truncation
    let final_lines: Vec<String> = processed_lines
        .iter()
        .map(|line| truncate_line(line, config))
        .collect();

    let result = final_lines.join("\n");

    (result, truncated, truncated_lines)
}

/// Truncate a single line if it exceeds the maximum length
fn truncate_line(line: &str, config: &TruncationConfig) -> String {
    if config.max_line_length == 0 {
        return line.to_string();
    }

    // Don't truncate our own truncation markers
    if line.contains(LINES_OMITTED_MARKER) {
        return line.to_string();
    }

    let char_count = line.chars().count();
    if char_count <= config.max_line_length {
        return line.to_string();
    }

    let chars: Vec<char> = line.chars().collect();
    let head_count = config.head_chars.min(chars.len());
    let tail_count = config
        .tail_chars
        .min(chars.len().saturating_sub(head_count));

    let head: String = chars[..head_count].iter().collect();
    let tail_start = chars.len().saturating_sub(tail_count);
    let tail: String = chars[tail_start..].iter().collect();

    let omitted = chars.len().saturating_sub(head_count + tail_count);

    if omitted > 0 {
        format!("{}[...{} chars omitted...]{}", head, omitted, tail)
    } else {
        line.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> TruncationConfig {
        TruncationConfig {
            max_lines: 100,
            head_lines: 20,
            tail_lines: 10,
            max_line_length: 500,
            head_chars: 200,
            tail_chars: 100,
        }
    }

    #[test]
    fn test_no_truncation_needed() {
        let config = default_config();
        let content = "line1\nline2\nline3";
        let (result, truncated, removed) = truncate_content(content, &config);

        assert!(!truncated);
        assert_eq!(removed, 0);
        assert_eq!(result, content);
    }

    #[test]
    fn test_line_truncation() {
        let config = TruncationConfig {
            max_lines: 10,
            head_lines: 3,
            tail_lines: 2,
            ..default_config()
        };

        let lines: Vec<String> = (1..=20).map(|i| format!("line{}", i)).collect();
        let content = lines.join("\n");
        let (result, truncated, removed) = truncate_content(&content, &config);

        assert!(truncated);
        assert_eq!(removed, 15); // 20 - 3 - 2
        assert!(result.contains("line1"));
        assert!(result.contains("line2"));
        assert!(result.contains("line3"));
        assert!(result.contains("15 lines omitted"));
        assert!(result.contains("line19"));
        assert!(result.contains("line20"));
        assert!(!result.contains("line10"));
    }

    #[test]
    fn test_line_length_truncation() {
        let config = TruncationConfig {
            max_line_length: 20,
            head_chars: 5,
            tail_chars: 5,
            ..default_config()
        };

        let long_line = "abcdefghijklmnopqrstuvwxyz0123456789";
        let (result, _, _) = truncate_content(long_line, &config);

        assert!(result.contains("abcde"));
        assert!(result.contains("chars omitted"));
        assert!(result.contains("56789"));
    }

    #[test]
    fn test_disabled_truncation() {
        let config = TruncationConfig {
            max_lines: 0,       // Disabled
            max_line_length: 0, // Disabled
            ..default_config()
        };

        let lines: Vec<String> = (1..=1000).map(|i| format!("line{}", i)).collect();
        let content = lines.join("\n");
        let (result, truncated, _) = truncate_content(&content, &config);

        assert!(!truncated);
        assert!(result.contains("line1000"));
    }

    #[test]
    fn test_truncation_marker_no_extra_newlines() {
        let config = TruncationConfig {
            max_lines: 5,
            head_lines: 2,
            tail_lines: 1,
            ..default_config()
        };

        let content = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10";
        let (result, truncated, _) = truncate_content(content, &config);

        assert!(truncated);
        // Check that there are no double newlines around the marker
        assert!(!result.contains("\n\n"));
    }

    #[test]
    fn test_combined_line_and_length_truncation() {
        let config = TruncationConfig {
            max_lines: 5,
            head_lines: 2,
            tail_lines: 1,
            max_line_length: 20,
            head_chars: 5,
            tail_chars: 5,
        };

        // Create content with long lines
        let long_line = "abcdefghijklmnopqrstuvwxyz0123456789";
        let lines: Vec<&str> = vec![long_line; 10];
        let content = lines.join("\n");

        let (result, truncated, _) = truncate_content(&content, &config);

        assert!(truncated);
        assert!(result.contains("lines omitted"));
        assert!(result.contains("chars omitted"));
    }

    #[test]
    fn test_marker_not_truncated() {
        // Ensure our truncation marker doesn't get line-length truncated
        let config = TruncationConfig {
            max_lines: 5,
            head_lines: 2,
            tail_lines: 1,
            max_line_length: 10, // Very short - would truncate the marker if not protected
            head_chars: 3,
            tail_chars: 3,
        };

        let lines: Vec<String> = (1..=20).map(|i| format!("line{}", i)).collect();
        let content = lines.join("\n");
        let (result, _, _) = truncate_content(&content, &config);

        // The marker should be intact, not truncated
        assert!(result.contains("lines omitted"));
    }
}
