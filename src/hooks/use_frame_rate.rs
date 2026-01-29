//! Frame rate statistics hook
//!
//! This module provides the `use_frame_rate` hook for accessing
//! frame rate statistics from within components.

use crate::renderer::FrameRateStats;

/// Hook to access frame rate statistics.
///
/// Returns `None` if frame rate statistics collection is not enabled.
/// Enable it with `.collect_frame_stats()` on the `AppBuilder`.
///
/// # Example
///
/// ```ignore
/// use rnk::prelude::*;
///
/// fn my_component() -> Element {
///     if let Some(stats) = use_frame_rate() {
///         Text::new(format!("FPS: {:.1}", stats.current_fps)).into_element()
///     } else {
///         Text::new("Stats not enabled").into_element()
///     }
/// }
///
/// // Enable stats collection
/// render(my_component)
///     .collect_frame_stats()
///     .run()?;
/// ```
pub fn use_frame_rate() -> Option<FrameRateStats> {
    // Get from RuntimeContext
    if let Some(ctx) = crate::runtime::current_runtime() {
        let borrowed = ctx.borrow();
        if let Some(stats) = borrowed.frame_rate_stats() {
            return Some(stats.snapshot());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_frame_rate_returns_none_when_not_set() {
        // Clear any existing runtime
        crate::runtime::set_current_runtime(None);

        assert!(use_frame_rate().is_none());
    }

    #[test]
    fn test_use_frame_rate_with_runtime() {
        use crate::renderer::SharedFrameRateStats;
        use crate::runtime::{RuntimeContext, with_runtime};
        use std::cell::RefCell;
        use std::rc::Rc;

        let stats = SharedFrameRateStats::new();
        let ctx = Rc::new(RefCell::new(RuntimeContext::new()));
        ctx.borrow_mut().set_frame_rate_stats(Some(stats));

        with_runtime(ctx.clone(), || {
            let result = use_frame_rate();
            assert!(result.is_some());
        });
    }
}
