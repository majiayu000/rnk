//! Bridge between Taffy's measurement protocol and the shared text flow.
//!
//! Split out of `engine.rs`, which was already over the file size ceiling.

use taffy::AvailableSpace;

use crate::core::TextWrap;
use crate::layout::measure::measure_text_width;
use crate::layout::text_flow::flow_text;

/// Per-node data Taffy hands back to the measure callback.
pub(super) struct NodeContext {
    pub(super) text_content: Option<String>,
    /// Measurement must see the wrap mode the renderer will use, or reserved
    /// height and painted rows can disagree.
    pub(super) text_wrap: TextWrap,
    cache: Option<CacheEntry>,
}

impl NodeContext {
    pub(super) fn new(text_content: Option<String>, text_wrap: TextWrap) -> Self {
        Self {
            text_content,
            text_wrap,
            cache: None,
        }
    }
}

/// A measurement together with the inputs that produced it.
#[derive(Clone, Copy)]
struct CacheEntry {
    known_width: Option<f32>,
    known_height: Option<f32>,
    available_width: AvailableWidthKey,
    size: taffy::Size<f32>,
}

/// The part of `AvailableSpace` that changes a text measurement.
///
/// `AvailableSpace` is not `PartialEq`, and the height axis never affects the
/// result, so the key carries only what matters.
#[derive(Clone, Copy, PartialEq)]
enum AvailableWidthKey {
    Definite(f32),
    MinContent,
    MaxContent,
}

impl From<AvailableSpace> for AvailableWidthKey {
    fn from(space: AvailableSpace) -> Self {
        match space {
            AvailableSpace::Definite(width) => Self::Definite(width),
            AvailableSpace::MinContent => Self::MinContent,
            AvailableSpace::MaxContent => Self::MaxContent,
        }
    }
}

/// Measure a text node for Taffy.
///
/// Results are memoised per node. Taffy probes each node at min-content, then
/// max-content, then the resolved width, and a parent's probing re-drives its
/// whole subtree — so without memoisation the repeated work compounds with
/// nesting depth and a deep tree becomes unusably slow.
pub(super) fn measure_text_node(
    known_dimensions: taffy::Size<Option<f32>>,
    available_space: taffy::Size<AvailableSpace>,
    node_context: Option<&mut NodeContext>,
) -> taffy::Size<f32> {
    let fallback = taffy::Size {
        width: known_dimensions.width.unwrap_or(0.0),
        height: known_dimensions.height.unwrap_or(0.0),
    };

    let Some(context) = node_context else {
        return fallback;
    };
    if context.text_content.as_deref().is_none_or(str::is_empty) {
        return fallback;
    }

    let available_width = AvailableWidthKey::from(available_space.width);
    if let Some(hit) = context.cache.filter(|entry| {
        entry.known_width == known_dimensions.width
            && entry.known_height == known_dimensions.height
            && entry.available_width == available_width
    }) {
        return hit.size;
    }

    let text_wrap = context.text_wrap;
    let text = context.text_content.as_deref().unwrap_or_default();

    let text_width = measure_text_width(text) as f32;

    let definite_width = match available_space.width {
        AvailableSpace::Definite(width) => Some(width as usize),
        _ => None,
    };

    // Height comes from the same flow the renderer will draw.
    let text_height = match definite_width {
        Some(max_width) if max_width > 0 => {
            flow_text(text, max_width, text_wrap).row_count() as f32
        }
        _ => text.lines().count().max(1) as f32,
    };

    let width = known_dimensions
        .width
        .unwrap_or_else(|| match available_space.width {
            AvailableSpace::Definite(width) => text_width.min(width),
            AvailableSpace::MinContent => text_width,
            AvailableSpace::MaxContent => text_width,
        });

    let size = taffy::Size {
        width,
        height: known_dimensions.height.unwrap_or(text_height),
    };

    context.cache = Some(CacheEntry {
        known_width: known_dimensions.width,
        known_height: known_dimensions.height,
        available_width,
        size,
    });

    size
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definite(width: f32) -> taffy::Size<AvailableSpace> {
        taffy::Size {
            width: AvailableSpace::Definite(width),
            height: AvailableSpace::MaxContent,
        }
    }

    fn unknown() -> taffy::Size<Option<f32>> {
        taffy::Size {
            width: None,
            height: None,
        }
    }

    #[test]
    fn wrapped_height_counts_every_row() {
        let mut ctx = NodeContext::new(Some("aaaa bbbb cccc dddd".into()), TextWrap::Wrap);
        let size = measure_text_node(unknown(), definite(10.0), Some(&mut ctx));
        assert_eq!(size.height, 2.0);
    }

    #[test]
    fn repeated_measurement_with_the_same_inputs_is_cached() {
        let mut ctx = NodeContext::new(Some("aaaa bbbb cccc dddd".into()), TextWrap::Wrap);
        let first = measure_text_node(unknown(), definite(10.0), Some(&mut ctx));
        assert!(ctx.cache.is_some(), "first call should populate the cache");

        let second = measure_text_node(unknown(), definite(10.0), Some(&mut ctx));
        assert_eq!((first.width, first.height), (second.width, second.height));
    }

    #[test]
    fn a_different_available_width_is_not_a_cache_hit() {
        let mut ctx = NodeContext::new(Some("aaaa bbbb cccc dddd".into()), TextWrap::Wrap);
        let narrow = measure_text_node(unknown(), definite(10.0), Some(&mut ctx));
        let wide = measure_text_node(unknown(), definite(40.0), Some(&mut ctx));
        assert_eq!(narrow.height, 2.0);
        assert_eq!(wide.height, 1.0, "wider box should re-measure, not reuse");
    }

    #[test]
    fn min_content_stays_at_the_full_line_width() {
        // Shrinking is enabled by `min-width: 0` on the node, not by
        // understating min-content here. Narrowing this value makes Taffy's
        // sizing search explode on nested trees, so it is load-bearing.
        let mut ctx = NodeContext::new(Some("aaaa bbbb cccc".into()), TextWrap::Wrap);
        let size = measure_text_node(
            unknown(),
            taffy::Size {
                width: AvailableSpace::MinContent,
                height: AvailableSpace::MaxContent,
            },
            Some(&mut ctx),
        );
        assert_eq!(size.width, 14.0);
    }

    #[test]
    fn empty_text_measures_to_the_known_dimensions() {
        let mut ctx = NodeContext::new(Some(String::new()), TextWrap::Wrap);
        let size = measure_text_node(unknown(), definite(10.0), Some(&mut ctx));
        assert_eq!((size.width, size.height), (0.0, 0.0));
    }
}
