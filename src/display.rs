use crate::api::TreeNode;

pub fn print_tree(tree: &[TreeNode], level: usize) {
    let indent = "  ".repeat(level);
    for node in tree {
        if node.r#type == "tree" {
            println!("{}{}/", indent, node.path);
            // Recursively print subdirectories
            let subdir_tree: Vec<TreeNode> = tree.iter()
                .filter_map(|n| {
                    if n.path.starts_with(&node.path) && n.path != node.path {
                        Some(n.clone())
                    } else {
                        None
                    }
                })
                .collect();
            print_tree(&subdir_tree, level + 1);
        } else if node.r#type == "blob" {
            println!("{}{}", indent, node.path);
        }
    }
}
