//! Content formatting for different output formats.

use crate::cli::ContentFormat;
use crate::config::Config;
use crate::content::FileEntry;
use crate::error::PctxError;
use crate::output::tree;
use std::path::PathBuf;

/// Format file entries according to configuration
pub fn format_output(entries: &[FileEntry], config: &Config) -> Result<String, PctxError> {
    match config.output_format {
        ContentFormat::Markdown => format_markdown(entries, config),
        ContentFormat::Xml => format_xml(entries, config),
        ContentFormat::Plain => format_plain(entries, config),
    }
}

/// Determine the minimum fence string that doesn't appear in the content
fn fence_for_content(content: &str) -> String {
    let mut fence = "```".to_string();
    while content.contains(&fence) {
        fence.push('`');
    }
    fence
}

fn format_markdown(entries: &[FileEntry], config: &Config) -> Result<String, PctxError> {
    let mut output = String::new();

    // Optional tree view
    if config.show_tree {
        let paths: Vec<_> = entries
            .iter()
            .map(|e| PathBuf::from(&e.relative_path))
            .collect();
        let tree_struct = tree::build_tree(&paths);

        output.push_str("## File Tree\n\n```\n");
        output.push_str(&tree::tree_to_string(&tree_struct));
        output.push_str("```\n\n");
    }

    // File contents
    for entry in entries {
        let display_path = entry.display_path(config.absolute_paths);

        // Format: `path/to/file.ext`:
        output.push_str(&format!("`{}`:\n", display_path));

        // Code block with language
        let lang = extension_to_language(&entry.extension);
        let fence = fence_for_content(&entry.content);
        output.push_str(&format!("{}{}\n", fence, lang));
        output.push_str(&entry.content);

        // Ensure content ends with newline
        if !entry.content.ends_with('\n') {
            output.push('\n');
        }

        output.push_str(&fence);
        output.push_str("\n\n");
    }

    Ok(output)
}

fn format_xml(entries: &[FileEntry], config: &Config) -> Result<String, PctxError> {
    let mut output = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<context>\n");

    if config.show_tree {
        let paths: Vec<_> = entries
            .iter()
            .map(|e| PathBuf::from(&e.relative_path))
            .collect();
        let tree_struct = tree::build_tree(&paths);
        let tree_str = tree::tree_to_string(&tree_struct);
        output.push_str("  <tree><![CDATA[\n");
        output.push_str(&escape_cdata_content(&tree_str));
        output.push_str("]]></tree>\n");
    }

    for entry in entries {
        let display_path = entry.display_path(config.absolute_paths);

        // Escape path for XML attribute
        let escaped_path = escape_xml_attr(&display_path);
        output.push_str(&format!(
            "  <file path=\"{}\" language=\"{}\">\n",
            escaped_path,
            extension_to_language(&entry.extension)
        ));
        output.push_str("<![CDATA[\n");
        // Escape CDATA content to prevent injection
        output.push_str(&escape_cdata_content(&entry.content));
        if !entry.content.ends_with('\n') {
            output.push('\n');
        }
        output.push_str("]]>\n  </file>\n");
    }

    output.push_str("</context>\n");
    Ok(output)
}

fn format_plain(entries: &[FileEntry], config: &Config) -> Result<String, PctxError> {
    let mut output = String::new();

    if config.show_tree {
        let paths: Vec<_> = entries
            .iter()
            .map(|e| PathBuf::from(&e.relative_path))
            .collect();
        let tree_struct = tree::build_tree(&paths);
        output.push_str("=== File Tree ===\n");
        output.push_str(&tree::tree_to_string(&tree_struct));
        output.push('\n');
    }

    for entry in entries {
        let display_path = entry.display_path(config.absolute_paths);

        output.push_str(&format!("=== {} ===\n", display_path));
        output.push_str(&entry.content);
        if !entry.content.ends_with('\n') {
            output.push('\n');
        }
        output.push('\n');
    }

    Ok(output)
}

/// Escape special characters for XML attributes
fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Escape CDATA content to prevent CDATA injection
///
/// The sequence `]]>` would close the CDATA section prematurely, so we need to
/// split it into multiple CDATA sections: `]]` + `]]><![CDATA[` + `>`
fn escape_cdata_content(content: &str) -> String {
    content.replace("]]>", "]]]]><![CDATA[>")
}

/// Map file extension to markdown/syntax highlighting language
fn extension_to_language(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        // Rust
        "rs" => "rust",
        // Go
        "go" => "go",
        // Python
        "py" | "pyi" => "python",
        // JavaScript/TypeScript
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "jsx" => "jsx",
        "tsx" => "tsx",
        // JVM languages
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "scala" => "scala",
        "groovy" | "gradle" => "groovy",
        // C/C++
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => "cpp",
        // C#/F#
        "cs" => "csharp",
        "fs" | "fsx" => "fsharp",
        // Ruby
        "rb" | "rake" | "gemspec" => "ruby",
        // PHP
        "php" | "phtml" => "php",
        // Shell
        "sh" | "bash" => "bash",
        "zsh" => "zsh",
        "fish" => "fish",
        "ps1" | "psm1" => "powershell",
        "bat" | "cmd" => "batch",
        // Web
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "sass" => "sass",
        "less" => "less",
        // Data formats
        "json" | "jsonc" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" | "xsl" | "xslt" => "xml",
        "csv" => "csv",
        // Documentation
        "md" | "markdown" => "markdown",
        "rst" => "rst",
        "tex" => "latex",
        // Query languages
        "sql" => "sql",
        "graphql" | "gql" => "graphql",
        // Config/DevOps
        "dockerfile" => "dockerfile",
        "makefile" | "mk" => "makefile",
        "tf" | "tfvars" => "hcl",
        // Frontend frameworks
        "vue" => "vue",
        "svelte" => "svelte",
        // Functional languages
        "hs" | "lhs" => "haskell",
        "elm" => "elm",
        "ml" | "mli" => "ocaml",
        "clj" | "cljs" | "cljc" => "clojure",
        "ex" | "exs" => "elixir",
        "erl" | "hrl" => "erlang",
        // Other languages
        "swift" => "swift",
        "m" | "mm" => "objective-c",
        "r" => "r",
        "jl" => "julia",
        "lua" => "lua",
        "vim" => "vim",
        "el" | "lisp" => "lisp",
        "dart" => "dart",
        "zig" => "zig",
        "nim" => "nim",
        // Protocol/Schema
        "proto" => "protobuf",
        "thrift" => "thrift",
        // Plain text and unknown extensions
        "txt" => "text",
        // Default: return empty string for unknown extensions
        // (syntax highlighters will treat as plain text)
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_cdata_content() {
        assert_eq!(escape_cdata_content("normal text"), "normal text");
        assert_eq!(
            escape_cdata_content("text with ]]> in it"),
            "text with ]]]]><![CDATA[> in it"
        );
        assert_eq!(
            escape_cdata_content("multiple ]]> and ]]> here"),
            "multiple ]]]]><![CDATA[> and ]]]]><![CDATA[> here"
        );
    }

    #[test]
    fn test_escape_xml_attr() {
        assert_eq!(escape_xml_attr("normal"), "normal");
        assert_eq!(escape_xml_attr("<test>"), "&lt;test&gt;");
        assert_eq!(escape_xml_attr("a&b"), "a&amp;b");
        assert_eq!(escape_xml_attr("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_extension_to_language() {
        assert_eq!(extension_to_language("rs"), "rust");
        assert_eq!(extension_to_language("RS"), "rust"); // case insensitive
        assert_eq!(extension_to_language("py"), "python");
        assert_eq!(extension_to_language("unknown_ext"), "");
        assert_eq!(extension_to_language(""), "");
    }

    // ==================== Snapshot tests for format_output ====================
    //
    // These snapshots are stored alongside this file (see src/output/snapshots/).
    // To regenerate after an intentional format change:
    //   cargo insta test --review      (interactive)
    //   INSTA_UPDATE=always cargo test  (accept all)

    use crate::config::TruncationConfig;

    fn entry(rel: &str, ext: &str, content: &str) -> FileEntry {
        FileEntry {
            absolute_path: std::path::PathBuf::from("/abs").join(rel),
            relative_path: rel.to_string(),
            extension: ext.to_string(),
            original_bytes: content.len(),
            original_lines: content.lines().count(),
            line_count: content.lines().count(),
            truncated: false,
            truncated_lines: 0,
            content: content.to_string(),
        }
    }

    fn config_with(format: ContentFormat, tree: bool) -> Config {
        Config {
            paths: vec![std::path::PathBuf::from(".")],
            exclude_patterns: Vec::new(),
            include_patterns: Vec::new(),
            include_hidden: false,
            use_default_excludes: true,
            use_gitignore: true,
            max_file_size: 1024 * 1024,
            max_depth: None,
            truncation: TruncationConfig::default(),
            output_format: format,
            show_tree: tree,
            show_stats: false,
            absolute_paths: false,
            verbose: false,
            quiet: false,
        }
    }

    #[test]
    fn snapshot_markdown_two_files() {
        let entries = vec![
            entry(
                "src/lib.rs",
                "rs",
                "pub fn hello() {\n    println!(\"hi\");\n}\n",
            ),
            entry("README.md", "md", "# Title\n\nBody.\n"),
        ];
        let cfg = config_with(ContentFormat::Markdown, false);
        let output = format_output(&entries, &cfg).unwrap();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_markdown_with_tree() {
        let entries = vec![
            entry("src/main.rs", "rs", "fn main() {}\n"),
            entry("src/lib.rs", "rs", "pub fn a() {}\n"),
            entry("README.md", "md", "# R\n"),
        ];
        let cfg = config_with(ContentFormat::Markdown, true);
        let output = format_output(&entries, &cfg).unwrap();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_markdown_fence_grows_for_triple_backtick() {
        let entries = vec![entry("doc.md", "md", "Example:\n```\ncode\n```\nEnd.\n")];
        let cfg = config_with(ContentFormat::Markdown, false);
        let output = format_output(&entries, &cfg).unwrap();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_markdown_fence_grows_for_quadruple_backtick() {
        let entries = vec![entry("doc.md", "md", "Nested:\n````\ninner\n````\nEnd.\n")];
        let cfg = config_with(ContentFormat::Markdown, false);
        let output = format_output(&entries, &cfg).unwrap();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_markdown_content_without_trailing_newline() {
        let entries = vec![entry("a.txt", "txt", "no trailing newline")];
        let cfg = config_with(ContentFormat::Markdown, false);
        let output = format_output(&entries, &cfg).unwrap();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_xml_two_files() {
        let entries = vec![
            entry("src/lib.rs", "rs", "pub fn hello() {}\n"),
            entry("data.json", "json", "{\"k\": 1}\n"),
        ];
        let cfg = config_with(ContentFormat::Xml, false);
        let output = format_output(&entries, &cfg).unwrap();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_xml_escapes_cdata_injection() {
        let entries = vec![entry("evil.txt", "txt", "before ]]> middle ]]> end\n")];
        let cfg = config_with(ContentFormat::Xml, false);
        let output = format_output(&entries, &cfg).unwrap();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_xml_escapes_path_attribute() {
        let entries = vec![entry("weird<&\"path>.txt", "txt", "hi\n")];
        let cfg = config_with(ContentFormat::Xml, false);
        let output = format_output(&entries, &cfg).unwrap();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_plain_two_files() {
        let entries = vec![
            entry("a.txt", "txt", "alpha\n"),
            entry("b.txt", "txt", "bravo"),
        ];
        let cfg = config_with(ContentFormat::Plain, false);
        let output = format_output(&entries, &cfg).unwrap();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_plain_with_tree() {
        let entries = vec![
            entry("src/a.txt", "txt", "alpha\n"),
            entry("src/b.txt", "txt", "bravo\n"),
        ];
        let cfg = config_with(ContentFormat::Plain, true);
        let output = format_output(&entries, &cfg).unwrap();
        insta::assert_snapshot!(output);
    }
}
