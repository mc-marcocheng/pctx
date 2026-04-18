//! File tree visualization.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Tree node structure
#[derive(Debug, Default)]
pub struct TreeNode {
    pub children: BTreeMap<String, TreeNode>,
    pub is_file: bool,
}

/// Build a tree structure from a list of paths
pub fn build_tree(paths: &[PathBuf]) -> TreeNode {
    let mut root = TreeNode::default();

    for path in paths {
        let mut current = &mut root;

        let components: Vec<_> = path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();
        let len = components.len();

        for (i, component) in components.into_iter().enumerate() {
            current = current
                .children
                .entry(component)
                .or_insert_with(TreeNode::default);

            if i == len - 1 {
                current.is_file = true;
            }
        }
    }

    root
}

/// Convert tree to string representation
pub fn tree_to_string(node: &TreeNode) -> String {
    let mut output = String::new();
    tree_to_string_inner(node, "", true, &mut output);
    output
}

fn tree_to_string_inner(node: &TreeNode, prefix: &str, is_root: bool, output: &mut String) {
    let entries: Vec<_> = node.children.iter().collect();
    let len = entries.len();

    for (i, (name, child)) in entries.iter().enumerate() {
        let is_last = i == len - 1;
        let connector = if is_root {
            ""
        } else if is_last {
            "└── "
        } else {
            "├── "
        };

        let child_prefix = if is_root {
            String::new()
        } else if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };

        if !is_root {
            output.push_str(&format!("{}{}{}\n", prefix, connector, name));
        } else if !name.is_empty() {
            output.push_str(&format!("{}\n", name));
        }

        if !child.children.is_empty() {
            tree_to_string_inner(child, &child_prefix, false, output);
        }
    }
}

/// Print tree to stderr
pub fn print_tree(node: &TreeNode) {
    eprint!("{}", tree_to_string(node));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_tree() {
        let paths = vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/lib.rs"),
            PathBuf::from("src/config/mod.rs"),
            PathBuf::from("Cargo.toml"),
        ];

        let tree = build_tree(&paths);

        assert!(tree.children.contains_key("src"));
        assert!(tree.children.contains_key("Cargo.toml"));

        let src = &tree.children["src"];
        assert!(src.children.contains_key("main.rs"));
        assert!(src.children.contains_key("lib.rs"));
        assert!(src.children.contains_key("config"));
    }
}