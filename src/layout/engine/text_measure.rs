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

    pub(super) fn update_text_wrap(&mut self, text_wrap: TextWrap) {
        if self.text_wrap != text_wrap {
            self.text_wrap = text_wrap;
            self.cache = None;
        }
    }
}

/// A measurement together with the inputs that produced it.
#[derive(Clone, Copy)]
struct CacheEntry {
    effective_width: EffectiveWidth,
    known_height: Option<f32>,
    text_wrap: TextWrap,
    size: taffy::Size<f32>,
}

/// Width that controls both wrapping and the returned measurement.
///
/// Resolved and available widths retain their provenance because a resolved
/// width is authoritative, while an available width only caps intrinsic text
/// width. Intrinsic probes have no finite wrapping width.
#[derive(Clone, Copy, PartialEq)]
enum EffectiveWidth {
    Resolved(f32),
    Available(f32),
    Intrinsic,
}

impl EffectiveWidth {
    fn value(self) -> Option<f32> {
        match self {
            Self::Resolved(width) | Self::Available(width) => Some(width),
            Self::Intrinsic => None,
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

    let effective_width = match (known_dimensions.width, available_space.width) {
        (Some(width), _) => EffectiveWidth::Resolved(width),
        (None, AvailableSpace::Definite(width)) => EffectiveWidth::Available(width),
        (None, AvailableSpace::MinContent | AvailableSpace::MaxContent) => {
            EffectiveWidth::Intrinsic
        }
    };
    let text_wrap = context.text_wrap;
    if let Some(hit) = context.cache.filter(|entry| {
        entry.effective_width == effective_width
            && entry.known_height == known_dimensions.height
            && entry.text_wrap == text_wrap
    }) {
        return hit.size;
    }

    let text = context.text_content.as_deref().unwrap_or_default();

    let text_width = measure_text_width(text) as f32;

    // Height comes from the same flow the renderer will draw.
    let text_height = match effective_width.value() {
        Some(width) if width > 0.0 => flow_text(text, width as usize, text_wrap).row_count() as f32,
        _ => text.lines().count().max(1) as f32,
    };

    let width = match effective_width {
        EffectiveWidth::Resolved(width) => width,
        EffectiveWidth::Available(width) => text_width.min(width),
        EffectiveWidth::Intrinsic => text_width,
    };

    let size = taffy::Size {
        width,
        // A resolved height remains authoritative even though wrapping is
        // always derived from the effective width above.
        height: known_dimensions.height.unwrap_or(text_height),
    };

    context.cache = Some(CacheEntry {
        effective_width,
        known_height: known_dimensions.height,
        text_wrap,
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
    fn known_width_drives_wrapped_height() {
        let mut ctx = NodeContext::new(Some("abcdefghijabcdefghij".into()), TextWrap::Wrap);
        let size = measure_text_node(
            taffy::Size {
                width: Some(10.0),
                height: None,
            },
            definite(80.0),
            Some(&mut ctx),
        );

        assert_eq!(
            (size.width, size.height),
            (10.0, 2.0),
            "height must describe the same resolved width returned to Taffy"
        );
    }

    #[test]
    fn different_known_widths_do_not_share_measurement() {
        let mut ctx = NodeContext::new(Some("abcdefghijabcdefghij".into()), TextWrap::Wrap);
        let narrow = measure_text_node(
            taffy::Size {
                width: Some(10.0),
                height: None,
            },
            definite(80.0),
            Some(&mut ctx),
        );
        let wide = measure_text_node(
            taffy::Size {
                width: Some(20.0),
                height: None,
            },
            definite(80.0),
            Some(&mut ctx),
        );

        assert_eq!((narrow.width, narrow.height), (10.0, 2.0));
        assert_eq!((wide.width, wide.height), (20.0, 1.0));
    }

    #[test]
    fn known_height_remains_authoritative_at_effective_width() {
        let mut ctx = NodeContext::new(Some("abcdefghijabcdefghij".into()), TextWrap::Wrap);
        let constrained = measure_text_node(
            taffy::Size {
                width: Some(10.0),
                height: Some(7.0),
            },
            definite(80.0),
            Some(&mut ctx),
        );
        let unconstrained = measure_text_node(
            taffy::Size {
                width: Some(10.0),
                height: None,
            },
            definite(80.0),
            Some(&mut ctx),
        );

        assert_eq!((constrained.width, constrained.height), (10.0, 7.0));
        assert_eq!((unconstrained.width, unconstrained.height), (10.0, 2.0));
    }

    #[test]
    fn no_known_width_keeps_definite_available_width_behavior() {
        let mut ctx = NodeContext::new(Some("abcdefghijabcdefghij".into()), TextWrap::Wrap);
        let size = measure_text_node(unknown(), definite(10.0), Some(&mut ctx));

        assert_eq!((size.width, size.height), (10.0, 2.0));
    }

    #[test]
    fn resolved_and_available_widths_do_not_share_returned_size() {
        let mut ctx = NodeContext::new(Some("abcde".into()), TextWrap::Wrap);
        let resolved = measure_text_node(
            taffy::Size {
                width: Some(20.0),
                height: None,
            },
            definite(80.0),
            Some(&mut ctx),
        );
        let available = measure_text_node(unknown(), definite(20.0), Some(&mut ctx));

        assert_eq!((resolved.width, resolved.height), (20.0, 1.0));
        assert_eq!((available.width, available.height), (5.0, 1.0));
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
    fn wrap_change_invalidates_only_affected_measurement() {
        let source = Some("abcdefgh".to_owned());
        let mut ctx = NodeContext::new(source.clone(), TextWrap::Wrap);
        let wrapped = measure_text_node(unknown(), definite(4.0), Some(&mut ctx));
        assert_eq!(wrapped.height, 2.0);
        assert!(ctx.cache.is_some());

        ctx.update_text_wrap(TextWrap::Wrap);
        assert!(
            ctx.cache.is_some(),
            "an unchanged wrap mode must preserve its valid measurement"
        );
        assert_eq!(ctx.text_content, source);

        ctx.update_text_wrap(TextWrap::Truncate);
        assert_eq!(ctx.text_wrap, TextWrap::Truncate);
        assert!(
            ctx.cache.is_none(),
            "a changed wrap mode must invalidate the old measurement"
        );
        assert_eq!(ctx.text_content, source);

        let truncated = measure_text_node(unknown(), definite(4.0), Some(&mut ctx));
        assert_eq!(truncated.height, 1.0);
        assert!(ctx.cache.is_some());

        ctx.update_text_wrap(TextWrap::Truncate);
        assert!(
            ctx.cache.is_some(),
            "the replacement measurement stays valid while the mode is unchanged"
        );

        ctx.update_text_wrap(TextWrap::Wrap);
        assert!(ctx.cache.is_none());
        let wrapped_again = measure_text_node(unknown(), definite(4.0), Some(&mut ctx));
        assert_eq!(wrapped_again.height, 2.0);
        assert_eq!(ctx.text_content, source);
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
