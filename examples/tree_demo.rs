//! Tree demo - Hierarchical data display
//!
//! Run: cargo run --example tree_demo

use rnk::components::{Tree, TreeNode, TreeState, TreeStyle};

fn main() {
    println!("=== Tree Component Demo ===\n");

    // Create a sample tree
    let tree = create_sample_tree();

    println!("--- TreeNode API ---");
    println!("TreeNode::new(id, label) - Create a node");
    println!("TreeNode::leaf(id, label) - Create a leaf node");
    println!("TreeNode::with_data(id, label, data) - Create with data");
    println!(".child(node) - Add a child");
    println!(".children(iter) - Add multiple children");
    println!();

    println!("Sample tree structure:");
    println!("  node_count: {}", tree.node_count());
    println!("  is_leaf: {}", tree.is_leaf());
    println!("  has_children: {}", tree.has_children());
    println!();

    // TreeState
    println!("--- TreeState API ---");
    let mut state = TreeState::new(&tree);
    println!("TreeState::new(&tree):");
    println!("  visible_count: {} (only root)", state.visible_count());
    println!("  cursor: {}", state.cursor());
    println!("  focused: {:?}", state.focused());
    println!();

    // Expand root
    state.expand("root");
    state.rebuild_visible(&tree);
    println!("After expand(\"root\"):");
    println!("  visible_count: {}", state.visible_count());
    println!();

    // Expand all
    state.expand_all(&tree);
    state.rebuild_visible(&tree);
    println!("After expand_all():");
    println!("  visible_count: {} (all nodes)", state.visible_count());
    println!();

    // Navigation
    println!("--- Navigation ---");
    state.cursor_down();
    println!("cursor_down(): focused = {:?}", state.focused());

    state.cursor_down();
    state.cursor_down();
    println!("cursor_down() x2: focused = {:?}", state.focused());

    state.cursor_up();
    println!("cursor_up(): focused = {:?}", state.focused());

    state.cursor_last();
    println!("cursor_last(): focused = {:?}", state.focused());

    state.cursor_first();
    println!("cursor_first(): focused = {:?}", state.focused());
    println!();

    // Toggle
    println!("--- Toggle Expand/Collapse ---");
    state.toggle("src");
    state.rebuild_visible(&tree);
    println!("toggle(\"src\"): visible_count = {}", state.visible_count());

    state.toggle("src");
    state.rebuild_visible(&tree);
    println!(
        "toggle(\"src\") again: visible_count = {}",
        state.visible_count()
    );
    println!();

    // Style presets
    println!("--- Style Presets ---");
    println!();

    // Default style
    println!("TreeStyle::default() (arrows):");
    print_tree_visual(
        &tree,
        &TreeState::all_expanded(&tree),
        &TreeStyle::default(),
    );

    // Folder icons
    println!("\nTreeStyle::folder_icons():");
    print_tree_visual(
        &tree,
        &TreeState::all_expanded(&tree),
        &TreeStyle::folder_icons(),
    );

    // Plus/minus
    println!("\nTreeStyle::plus_minus_icons():");
    print_tree_visual(
        &tree,
        &TreeState::all_expanded(&tree),
        &TreeStyle::plus_minus_icons(),
    );

    // Minimal
    println!("\nTreeStyle::minimal() (no lines):");
    print_tree_visual(
        &tree,
        &TreeState::all_expanded(&tree),
        &TreeStyle::minimal(),
    );

    // Collapsed view
    println!("\n--- Collapsed View ---");
    let mut state = TreeState::new(&tree);
    state.expand("root");
    state.rebuild_visible(&tree);
    print_tree_visual(&tree, &state, &TreeStyle::default());

    // Usage example
    println!("\n--- Usage in TUI App ---");
    println!("```rust");
    println!("use rnk::components::{{Tree, TreeNode, TreeState, handle_tree_input}};");
    println!("use rnk::hooks::{{use_signal, use_input}};");
    println!();
    println!("fn app() -> Element {{");
    println!("    let tree = TreeNode::new(\"root\", \"Project\")");
    println!("        .child(TreeNode::new(\"src\", \"src\")");
    println!("            .child(TreeNode::leaf(\"main\", \"main.rs\")))");
    println!("        .child(TreeNode::leaf(\"cargo\", \"Cargo.toml\"));");
    println!();
    println!("    let state = use_signal(|| TreeState::with_root_expanded(&tree));");
    println!();
    println!("    use_input(move |input, key| {{");
    println!("        let mut s = state.get();");
    println!("        if handle_tree_input(&mut s, &tree, input, key) {{");
    println!("            state.set(s);");
    println!("        }}");
    println!("    }});");
    println!();
    println!("    Tree::new(&tree, &state.get())");
    println!("        .style(TreeStyle::folder_icons())");
    println!("        .into_element()");
    println!("}}");
    println!("```");
}

fn create_sample_tree() -> TreeNode<()> {
    TreeNode::new("root", "my-project")
        .child(
            TreeNode::new("src", "src")
                .child(
                    TreeNode::new("components", "components")
                        .child(TreeNode::leaf("button", "button.rs"))
                        .child(TreeNode::leaf("input", "input.rs"))
                        .child(TreeNode::leaf("mod", "mod.rs")),
                )
                .child(TreeNode::leaf("lib", "lib.rs"))
                .child(TreeNode::leaf("main", "main.rs")),
        )
        .child(
            TreeNode::new("tests", "tests")
                .child(TreeNode::leaf("integration", "integration_test.rs")),
        )
        .child(TreeNode::leaf("cargo", "Cargo.toml"))
        .child(TreeNode::leaf("readme", "README.md"))
}

fn print_tree_visual(tree: &TreeNode<()>, state: &TreeState, style: &TreeStyle) {
    print_node(tree, state, style, 0, vec![]);
}

fn print_node(
    node: &TreeNode<()>,
    state: &TreeState,
    style: &TreeStyle,
    depth: usize,
    parent_is_last: Vec<bool>,
) {
    let is_expanded = state.is_expanded(&node.id);
    let is_focused = state.focused() == Some(&node.id);

    // Build prefix
    let mut prefix = String::new();
    if style.show_lines && depth > 0 {
        for &is_last in &parent_is_last[..parent_is_last.len().saturating_sub(1)] {
            if is_last {
                prefix.push_str("  ");
            } else {
                prefix.push_str(&style.vertical_line);
            }
        }
        if let Some(&is_last) = parent_is_last.last() {
            if is_last {
                prefix.push_str(&style.last_connector);
            } else {
                prefix.push_str(&style.connector);
            }
        }
    } else {
        prefix = "  ".repeat(depth);
    }

    // Icon
    let icon = if node.is_leaf() {
        &style.leaf_icon
    } else if is_expanded {
        &style.expanded_icon
    } else {
        &style.collapsed_icon
    };

    // Color
    let (color_start, color_end) = if is_focused {
        ("\x1b[1;36m", "\x1b[0m")
    } else if let Some(_) = style.icon_color {
        ("\x1b[36m", "\x1b[0m")
    } else {
        ("", "")
    };

    println!(
        "  {}{}{} {}{}",
        prefix, color_start, icon, node.label, color_end
    );

    // Children
    if is_expanded {
        let child_count = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            let is_last = i == child_count - 1;
            let mut child_is_last = parent_is_last.clone();
            child_is_last.push(is_last);
            print_node(child, state, style, depth + 1, child_is_last);
        }
    }
}
