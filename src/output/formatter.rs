//! Content formatting for different output formats.

use crate::cli::ContentFormat;
use crate::config::Config;
use crate::content::FileEntry;
use crate::error::PctxError;
use crate::output::tree;

/// Format file entries according to configuration
pub fn format_output(entries: &[FileEntry], config: &Config) -> Result<String, PctxError> {
    match config.output_format {
        ContentFormat::Markdown => format_markdown(entries, config),
        ContentFormat::Xml => format_xml(entries, config),
        ContentFormat::Plain => format_plain(entries, config),
    }
}

fn format_markdown(entries: &[FileEntry], config: &Config) -> Result<String, PctxError> {
    let mut output = String::new();

    // Optional tree view
    if config.show_tree {
        let paths: Vec<_> = entries.iter().map(|e| e.absolute_path.clone()).collect();
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
        output.push_str(&format!("```{}\n", lang));
        output.push_str(&entry.content);

        // Ensure content ends with newline
        if !entry.content.ends_with('\n') {
            output.push('\n');
        }

        output.push_str("```\n\n");
    }

    Ok(output)
}

fn format_xml(entries: &[FileEntry], config: &Config) -> Result<String, PctxError> {
    let mut output = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<context>\n");

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
}
