//! Layout engine using Taffy

use crate::core::{Element, ElementId, ElementType};
use crate::layout::measure::measure_text_width;
use std::collections::HashMap;
use taffy::{AvailableSpace, NodeId, TaffyTree};

/// Computed layout for an element
#[derive(Debug, Clone, Copy, Default)]
pub struct Layout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Context stored for each node (for text measurement)
#[derive(Clone)]
struct NodeContext {
    #[allow(dead_code)]
    element_id: ElementId,
    text_content: Option<String>,
}

/// Layout engine that computes element positions
pub struct LayoutEngine {
    taffy: TaffyTree<NodeContext>,
    node_map: HashMap<ElementId, NodeId>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            node_map: HashMap::new(),
        }
    }

    /// Build layout tree from element tree
    pub fn build_tree(&mut self, element: &Element) -> Option<NodeId> {
        self.taffy.clear();
        self.node_map.clear();
        self.build_node(element)
    }

    fn build_node(&mut self, element: &Element) -> Option<NodeId> {
        // Skip virtual text nodes (they don't have layout)
        if element.element_type == ElementType::VirtualText {
            return None;
        }

        let taffy_style = element.style.to_taffy();

        // Build children first
        let child_nodes: Vec<NodeId> = element
            .children
            .iter()
            .filter_map(|child| self.build_node(child))
            .collect();

        let context = NodeContext {
            element_id: element.id,
            text_content: element.text_content.clone(),
        };

        // Create node with measure function for text
        let node_id = if element.is_text() {
            self.taffy
                .new_leaf_with_context(taffy_style, context)
                .ok()?
        } else {
            let node = self
                .taffy
                .new_with_children(taffy_style, &child_nodes)
                .ok()?;
            // Set context for non-text nodes too
            let _ = self.taffy.set_node_context(node, Some(context));
            node
        };

        self.node_map.insert(element.id, node_id);
        Some(node_id)
    }

    /// Compute layout for the tree
    pub fn compute(&mut self, root: &Element, width: u16, height: u16) {
        if let Some(root_node) = self.build_tree(root) {
            let _ = self.taffy.compute_layout_with_measure(
                root_node,
                taffy::Size {
                    width: AvailableSpace::Definite(width as f32),
                    height: AvailableSpace::Definite(height as f32),
                },
                |known_dimensions, available_space, _node_id, node_context, _style| {
                    measure_text_node(known_dimensions, available_space, node_context)
                },
            );
        }
    }

    /// Get computed layout for an element
    pub fn get_layout(&self, element_id: ElementId) -> Option<Layout> {
        let node_id = self.node_map.get(&element_id)?;
        let layout = self.taffy.layout(*node_id).ok()?;

        Some(Layout {
            x: layout.location.x,
            y: layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        })
    }

    /// Get all layouts
    pub fn get_all_layouts(&self) -> HashMap<ElementId, Layout> {
        self.node_map
            .iter()
            .filter_map(|(element_id, node_id)| {
                let layout = self.taffy.layout(*node_id).ok()?;
                Some((
                    *element_id,
                    Layout {
                        x: layout.location.x,
                        y: layout.location.y,
                        width: layout.size.width,
                        height: layout.size.height,
                    },
                ))
            })
            .collect()
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Measure text content for layout
fn measure_text_node(
    known_dimensions: taffy::Size<Option<f32>>,
    available_space: taffy::Size<AvailableSpace>,
    node_context: Option<&mut NodeContext>,
) -> taffy::Size<f32> {
    let text = node_context
        .and_then(|ctx| ctx.text_content.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("");

    if text.is_empty() {
        return taffy::Size {
            width: known_dimensions.width.unwrap_or(0.0),
            height: known_dimensions.height.unwrap_or(0.0),
        };
    }

    // Measure text using unicode-width
    let text_width = measure_text_width(text) as f32;
    let text_height = text.lines().count().max(1) as f32;

    let width = known_dimensions
        .width
        .unwrap_or_else(|| match available_space.width {
            AvailableSpace::Definite(w) => text_width.min(w),
            AvailableSpace::MinContent => text_width,
            AvailableSpace::MaxContent => text_width,
        });

    let height = known_dimensions.height.unwrap_or(text_height);

    taffy::Size { width, height }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Element;

    #[test]
    fn test_layout_engine_creation() {
        let engine = LayoutEngine::new();
        assert!(engine.node_map.is_empty());
    }

    #[test]
    fn test_simple_layout() {
        let mut engine = LayoutEngine::new();

        let mut root = Element::root();
        root.add_child(Element::text("Hello"));

        engine.compute(&root, 80, 24);

        let layout = engine.get_layout(root.id);
        assert!(layout.is_some());
    }

    #[test]
    fn test_text_measurement() {
        let mut engine = LayoutEngine::new();

        let root = Element::text("Hello World");
        engine.compute(&root, 80, 24);

        let layout = engine.get_layout(root.id);
        assert!(layout.is_some());

        let layout = layout.unwrap();
        // "Hello World" is 11 characters wide
        assert!(layout.width >= 11.0);
    }
}
