//! Tree component for hierarchical data display
//!
//! A tree view component for displaying hierarchical data with
//! expand/collapse functionality.
//!
//! # Features
//!
//! - Hierarchical data display
//! - Expand/collapse nodes
//! - Keyboard navigation
//! - Customizable icons and indentation
//! - Selection support
//!
//! # Example
//!
//! ```ignore
//! use rnk::components::tree::{Tree, TreeNode, TreeState};
//! use rnk::hooks::{use_signal, use_input};
//!
//! fn app() -> Element {
//!     let root = TreeNode::new("root", "Root")
//!         .child(TreeNode::new("child1", "Child 1")
//!             .child(TreeNode::leaf("leaf1", "Leaf 1")))
//!         .child(TreeNode::leaf("child2", "Child 2"));
//!
//!     let state = use_signal(|| TreeState::new(&root));
//!
//!     Tree::new(&root, &state.get())
//!         .into_element()
//! }
//! ```

use crate::components::{Box as TinkBox, Text};
use crate::core::{Color, Element, FlexDirection};
use std::collections::HashSet;

/// A node in the tree
#[derive(Debug, Clone)]
pub struct TreeNode<T: Clone> {
    /// Unique identifier for this node
    pub id: String,
    /// Display label
    pub label: String,
    /// Associated data
    pub data: Option<T>,
    /// Child nodes
    pub children: Vec<TreeNode<T>>,
}

impl<T: Clone> TreeNode<T> {
    /// Create a new tree node with children capability
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            data: None,
            children: Vec::new(),
        }
    }

    /// Create a leaf node (no children)
    pub fn leaf(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(id, label)
    }

    /// Create a node with data
    pub fn with_data(id: impl Into<String>, label: impl Into<String>, data: T) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            data: Some(data),
            children: Vec::new(),
        }
    }

    /// Add a child node
    pub fn child(mut self, child: TreeNode<T>) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children
    pub fn children(mut self, children: impl IntoIterator<Item = TreeNode<T>>) -> Self {
        self.children.extend(children);
        self
    }

    /// Check if this is a leaf node
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Check if this node has children
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Get total node count (including self and all descendants)
    pub fn node_count(&self) -> usize {
        1 + self.children.iter().map(|c| c.node_count()).sum::<usize>()
    }

    /// Find a node by ID
    pub fn find(&self, id: &str) -> Option<&TreeNode<T>> {
        if self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find(id) {
                return Some(found);
            }
        }
        None
    }

    /// Get all node IDs in depth-first order
    pub fn all_ids(&self) -> Vec<String> {
        let mut ids = vec![self.id.clone()];
        for child in &self.children {
            ids.extend(child.all_ids());
        }
        ids
    }
}

/// Tree state for tracking expanded nodes and selection
#[derive(Debug, Clone)]
pub struct TreeState {
    /// Set of expanded node IDs
    expanded: HashSet<String>,
    /// Currently selected node ID
    selected: Option<String>,
    /// Flattened visible node IDs (for navigation)
    visible_nodes: Vec<String>,
    /// Current cursor position in visible nodes
    cursor: usize,
}

impl TreeState {
    /// Create a new tree state
    pub fn new<T: Clone>(root: &TreeNode<T>) -> Self {
        let mut state = Self {
            expanded: HashSet::new(),
            selected: None,
            visible_nodes: Vec::new(),
            cursor: 0,
        };
        state.rebuild_visible(root);
        state
    }

    /// Create with root expanded
    pub fn with_root_expanded<T: Clone>(root: &TreeNode<T>) -> Self {
        let mut state = Self::new(root);
        state.expanded.insert(root.id.clone());
        state.rebuild_visible(root);
        state
    }

    /// Create with all nodes expanded
    pub fn all_expanded<T: Clone>(root: &TreeNode<T>) -> Self {
        let mut state = Self::new(root);
        state.expand_all(root);
        state.rebuild_visible(root);
        state
    }

    /// Check if a node is expanded
    pub fn is_expanded(&self, id: &str) -> bool {
        self.expanded.contains(id)
    }

    /// Expand a node
    pub fn expand(&mut self, id: &str) {
        self.expanded.insert(id.to_string());
    }

    /// Collapse a node
    pub fn collapse(&mut self, id: &str) {
        self.expanded.remove(id);
    }

    /// Toggle expand/collapse
    pub fn toggle(&mut self, id: &str) {
        if self.expanded.contains(id) {
            self.expanded.remove(id);
        } else {
            self.expanded.insert(id.to_string());
        }
    }

    /// Expand all nodes
    pub fn expand_all<T: Clone>(&mut self, root: &TreeNode<T>) {
        for id in root.all_ids() {
            self.expanded.insert(id);
        }
    }

    /// Collapse all nodes
    pub fn collapse_all(&mut self) {
        self.expanded.clear();
    }

    /// Get selected node ID
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// Set selected node
    pub fn set_selected(&mut self, id: Option<String>) {
        self.selected = id;
    }

    /// Get cursor position
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Get currently focused node ID
    pub fn focused(&self) -> Option<&str> {
        self.visible_nodes.get(self.cursor).map(|s| s.as_str())
    }

    /// Move cursor up
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor down
    pub fn cursor_down(&mut self) {
        if self.cursor < self.visible_nodes.len().saturating_sub(1) {
            self.cursor += 1;
        }
    }

    /// Move cursor to first visible node
    pub fn cursor_first(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to last visible node
    pub fn cursor_last(&mut self) {
        self.cursor = self.visible_nodes.len().saturating_sub(1);
    }

    /// Toggle expand/collapse of focused node
    pub fn toggle_focused(&mut self) {
        if let Some(id) = self.focused().map(|s| s.to_string()) {
            self.toggle(&id);
        }
    }

    /// Select focused node
    pub fn select_focused(&mut self) {
        self.selected = self.focused().map(|s| s.to_string());
    }

    /// Rebuild visible nodes list based on expanded state
    pub fn rebuild_visible<T: Clone>(&mut self, root: &TreeNode<T>) {
        self.visible_nodes.clear();
        self.collect_visible(root);
        // Clamp cursor
        if self.cursor >= self.visible_nodes.len() {
            self.cursor = self.visible_nodes.len().saturating_sub(1);
        }
    }

    fn collect_visible<T: Clone>(&mut self, node: &TreeNode<T>) {
        self.visible_nodes.push(node.id.clone());
        if self.is_expanded(&node.id) {
            for child in &node.children {
                self.collect_visible(child);
            }
        }
    }

    /// Get visible node count
    pub fn visible_count(&self) -> usize {
        self.visible_nodes.len()
    }
}

/// Style configuration for the tree
#[derive(Debug, Clone)]
pub struct TreeStyle {
    /// Indentation per level (in spaces)
    pub indent: usize,
    /// Icon for expanded nodes
    pub expanded_icon: String,
    /// Icon for collapsed nodes
    pub collapsed_icon: String,
    /// Icon for leaf nodes
    pub leaf_icon: String,
    /// Connector for tree lines
    pub connector: String,
    /// Last item connector
    pub last_connector: String,
    /// Vertical line for tree structure
    pub vertical_line: String,
    /// Color for icons
    pub icon_color: Option<Color>,
    /// Color for selected item
    pub selected_color: Option<Color>,
    /// Color for focused item
    pub focused_color: Option<Color>,
    /// Background for focused item
    pub focused_bg: Option<Color>,
    /// Show tree lines
    pub show_lines: bool,
}

impl Default for TreeStyle {
    fn default() -> Self {
        Self {
            indent: 2,
            expanded_icon: "▼".to_string(),
            collapsed_icon: "▶".to_string(),
            leaf_icon: "•".to_string(),
            connector: "├─".to_string(),
            last_connector: "└─".to_string(),
            vertical_line: "│ ".to_string(),
            icon_color: Some(Color::Cyan),
            selected_color: Some(Color::Green),
            focused_color: Some(Color::Cyan),
            focused_bg: None,
            show_lines: true,
        }
    }
}

impl TreeStyle {
    /// Create a new style with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set indentation
    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    /// Set expanded icon
    pub fn expanded_icon(mut self, icon: impl Into<String>) -> Self {
        self.expanded_icon = icon.into();
        self
    }

    /// Set collapsed icon
    pub fn collapsed_icon(mut self, icon: impl Into<String>) -> Self {
        self.collapsed_icon = icon.into();
        self
    }

    /// Set leaf icon
    pub fn leaf_icon(mut self, icon: impl Into<String>) -> Self {
        self.leaf_icon = icon.into();
        self
    }

    /// Set icon color
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = Some(color);
        self
    }

    /// Set focused color
    pub fn focused_color(mut self, color: Color) -> Self {
        self.focused_color = Some(color);
        self
    }

    /// Enable/disable tree lines
    pub fn show_lines(mut self, show: bool) -> Self {
        self.show_lines = show;
        self
    }

    /// Use folder-style icons
    pub fn folder_icons() -> Self {
        Self {
            expanded_icon: "📂".to_string(),
            collapsed_icon: "📁".to_string(),
            leaf_icon: "📄".to_string(),
            ..Default::default()
        }
    }

    /// Use arrow-style icons
    pub fn arrow_icons() -> Self {
        Self {
            expanded_icon: "▼".to_string(),
            collapsed_icon: "▶".to_string(),
            leaf_icon: " ".to_string(),
            ..Default::default()
        }
    }

    /// Use plus/minus icons
    pub fn plus_minus_icons() -> Self {
        Self {
            expanded_icon: "[-]".to_string(),
            collapsed_icon: "[+]".to_string(),
            leaf_icon: " - ".to_string(),
            ..Default::default()
        }
    }

    /// Minimal style (no lines)
    pub fn minimal() -> Self {
        Self {
            show_lines: false,
            expanded_icon: "▾".to_string(),
            collapsed_icon: "▸".to_string(),
            leaf_icon: " ".to_string(),
            ..Default::default()
        }
    }
}

/// Tree component
#[derive(Debug, Clone)]
pub struct Tree<'a, T: Clone> {
    /// Root node
    root: &'a TreeNode<T>,
    /// Tree state
    state: &'a TreeState,
    /// Style configuration
    style: TreeStyle,
    /// Whether the tree is focused
    focused: bool,
}

impl<'a, T: Clone> Tree<'a, T> {
    /// Create a new tree
    pub fn new(root: &'a TreeNode<T>, state: &'a TreeState) -> Self {
        Self {
            root,
            state,
            style: TreeStyle::default(),
            focused: true,
        }
    }

    /// Set the style
    pub fn style(mut self, style: TreeStyle) -> Self {
        self.style = style;
        self
    }

    /// Set whether the tree is focused
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Convert to Element
    pub fn into_element(self) -> Element {
        let mut container = TinkBox::new().flex_direction(FlexDirection::Column);

        // Render tree recursively
        let elements = self.render_node(self.root, 0, vec![]);
        for elem in elements {
            container = container.child(elem);
        }

        container.into_element()
    }

    /// Render a node and its children
    fn render_node(
        &self,
        node: &TreeNode<T>,
        depth: usize,
        parent_is_last: Vec<bool>,
    ) -> Vec<Element> {
        let mut elements = Vec::new();

        let is_focused = self.focused && self.state.focused() == Some(&node.id);
        let is_selected = self.state.selected() == Some(&node.id);
        let is_expanded = self.state.is_expanded(&node.id);

        // Build prefix (tree lines)
        let mut prefix = String::new();
        if self.style.show_lines && depth > 0 {
            for &is_last in &parent_is_last[..parent_is_last.len().saturating_sub(1)] {
                if is_last {
                    prefix.push_str("  ");
                } else {
                    prefix.push_str(&self.style.vertical_line);
                }
            }
            if let Some(&is_last) = parent_is_last.last() {
                if is_last {
                    prefix.push_str(&self.style.last_connector);
                } else {
                    prefix.push_str(&self.style.connector);
                }
            }
        } else {
            prefix = " ".repeat(depth * self.style.indent);
        }

        // Build icon
        let icon = if node.is_leaf() {
            &self.style.leaf_icon
        } else if is_expanded {
            &self.style.expanded_icon
        } else {
            &self.style.collapsed_icon
        };

        // Build the line
        let line = format!("{}{} {}", prefix, icon, node.label);

        let mut text = Text::new(&line);

        // Apply styling
        if is_focused {
            if let Some(color) = self.style.focused_color {
                text = text.color(color);
            }
            if let Some(bg) = self.style.focused_bg {
                text = text.background(bg);
            }
            text = text.bold();
        } else if is_selected {
            if let Some(color) = self.style.selected_color {
                text = text.color(color);
            }
        }

        elements.push(text.into_element());

        // Render children if expanded
        if is_expanded {
            let child_count = node.children.len();
            for (i, child) in node.children.iter().enumerate() {
                let is_last = i == child_count - 1;
                let mut child_is_last = parent_is_last.clone();
                child_is_last.push(is_last);
                elements.extend(self.render_node(child, depth + 1, child_is_last));
            }
        }

        elements
    }
}

/// Handle tree input
pub fn handle_tree_input<T: Clone>(
    state: &mut TreeState,
    root: &TreeNode<T>,
    _input: &str,
    key: &crate::hooks::Key,
) -> bool {
    let mut handled = false;

    if key.up_arrow {
        state.cursor_up();
        handled = true;
    } else if key.down_arrow {
        state.cursor_down();
        handled = true;
    } else if key.left_arrow {
        // Collapse current node or move to parent
        if let Some(id) = state.focused().map(|s| s.to_string()) {
            if state.is_expanded(&id) {
                state.collapse(&id);
                state.rebuild_visible(root);
            }
        }
        handled = true;
    } else if key.right_arrow {
        // Expand current node
        if let Some(id) = state.focused().map(|s| s.to_string()) {
            if !state.is_expanded(&id) {
                state.expand(&id);
                state.rebuild_visible(root);
            }
        }
        handled = true;
    } else if key.return_key || key.space {
        // Toggle expand/collapse
        if let Some(id) = state.focused().map(|s| s.to_string()) {
            state.toggle(&id);
            state.rebuild_visible(root);
        }
        handled = true;
    } else if key.home {
        state.cursor_first();
        handled = true;
    } else if key.end {
        state.cursor_last();
        handled = true;
    }

    handled
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> TreeNode<()> {
        TreeNode::new("root", "Root")
            .child(
                TreeNode::new("a", "Node A")
                    .child(TreeNode::leaf("a1", "Leaf A1"))
                    .child(TreeNode::leaf("a2", "Leaf A2")),
            )
            .child(TreeNode::leaf("b", "Node B"))
            .child(TreeNode::new("c", "Node C").child(TreeNode::leaf("c1", "Leaf C1")))
    }

    #[test]
    fn test_tree_node_creation() {
        let node = TreeNode::<()>::new("test", "Test Node");
        assert_eq!(node.id, "test");
        assert_eq!(node.label, "Test Node");
        assert!(node.is_leaf());
    }

    #[test]
    fn test_tree_node_with_children() {
        let node = TreeNode::<()>::new("parent", "Parent")
            .child(TreeNode::leaf("child1", "Child 1"))
            .child(TreeNode::leaf("child2", "Child 2"));

        assert!(!node.is_leaf());
        assert!(node.has_children());
        assert_eq!(node.children.len(), 2);
    }

    #[test]
    fn test_tree_node_count() {
        let tree = sample_tree();
        assert_eq!(tree.node_count(), 7); // root + a + a1 + a2 + b + c + c1
    }

    #[test]
    fn test_tree_find() {
        let tree = sample_tree();
        assert!(tree.find("a1").is_some());
        assert!(tree.find("nonexistent").is_none());
    }

    #[test]
    fn test_tree_state_new() {
        let tree = sample_tree();
        let state = TreeState::new(&tree);

        assert_eq!(state.visible_count(), 1); // Only root visible
        assert_eq!(state.cursor(), 0);
        assert!(!state.is_expanded("root"));
    }

    #[test]
    fn test_tree_state_expand() {
        let tree = sample_tree();
        let mut state = TreeState::new(&tree);

        state.expand("root");
        state.rebuild_visible(&tree);

        assert!(state.is_expanded("root"));
        assert_eq!(state.visible_count(), 4); // root + a + b + c
    }

    #[test]
    fn test_tree_state_expand_all() {
        let tree = sample_tree();
        let mut state = TreeState::new(&tree);

        state.expand_all(&tree);
        state.rebuild_visible(&tree);

        assert_eq!(state.visible_count(), 7); // All nodes visible
    }

    #[test]
    fn test_tree_state_navigation() {
        let tree = sample_tree();
        let mut state = TreeState::all_expanded(&tree);

        assert_eq!(state.cursor(), 0);
        assert_eq!(state.focused(), Some("root"));

        state.cursor_down();
        assert_eq!(state.focused(), Some("a"));

        state.cursor_down();
        assert_eq!(state.focused(), Some("a1"));

        state.cursor_up();
        assert_eq!(state.focused(), Some("a"));

        state.cursor_last();
        assert_eq!(state.focused(), Some("c1"));

        state.cursor_first();
        assert_eq!(state.focused(), Some("root"));
    }

    #[test]
    fn test_tree_state_toggle() {
        let tree = sample_tree();
        let mut state = TreeState::new(&tree);

        state.expand("root");
        state.rebuild_visible(&tree);
        assert_eq!(state.visible_count(), 4);

        state.toggle("root");
        state.rebuild_visible(&tree);
        assert_eq!(state.visible_count(), 1);
    }

    #[test]
    fn test_tree_style_presets() {
        let _default = TreeStyle::default();
        let _folder = TreeStyle::folder_icons();
        let _arrow = TreeStyle::arrow_icons();
        let _plus_minus = TreeStyle::plus_minus_icons();
        let _minimal = TreeStyle::minimal();
    }
}
