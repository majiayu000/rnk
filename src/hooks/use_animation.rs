//! Animation hook for keyframe-based animations
//!
//! Provides a hook for running animations within components.

use crate::animation::{Animation, AnimationInstance, AnimationState};
use crate::hooks::context::{RenderCallback, current_context};
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Handle for controlling an animation
#[derive(Clone)]
pub struct AnimationHandle {
    instance: Arc<RwLock<AnimationInstance>>,
    last_tick: Arc<RwLock<Instant>>,
    render_callback: Option<RenderCallback>,
}

impl AnimationHandle {
    /// Get the current animated value
    pub fn get(&self) -> f32 {
        self.instance.read().unwrap().get()
    }

    /// Get the current value as i32
    pub fn get_i32(&self) -> i32 {
        self.instance.read().unwrap().get_i32()
    }

    /// Get the current value as usize
    pub fn get_usize(&self) -> usize {
        self.instance.read().unwrap().get_usize()
    }

    /// Start or resume the animation
    pub fn play(&self) {
        self.instance.write().unwrap().play();
        self.trigger_render();
    }

    /// Pause the animation
    pub fn pause(&self) {
        self.instance.write().unwrap().pause();
    }

    /// Reset the animation to the beginning
    pub fn reset(&self) {
        self.instance.write().unwrap().reset();
        self.trigger_render();
    }

    /// Check if the animation is running
    pub fn is_running(&self) -> bool {
        self.instance.read().unwrap().is_running()
    }

    /// Check if the animation has completed
    pub fn is_completed(&self) -> bool {
        self.instance.read().unwrap().is_completed()
    }

    /// Get the current animation state
    pub fn state(&self) -> AnimationState {
        self.instance.read().unwrap().state()
    }

    /// Get the animation progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        self.instance.read().unwrap().progress()
    }

    /// Tick the animation (called internally by the framework)
    pub fn tick(&self) {
        let now = Instant::now();
        let delta = {
            let mut last = self.last_tick.write().unwrap();
            let delta = now.duration_since(*last);
            *last = now;
            delta
        };

        let was_running = self.is_running();
        self.instance.write().unwrap().tick(delta);

        // Only trigger render if animation is still running
        if was_running && self.is_running() {
            self.trigger_render();
        }
    }

    fn trigger_render(&self) {
        if let Some(callback) = &self.render_callback {
            callback();
        }
    }

    // =========================================================================
    // Try methods (non-panicking versions)
    // =========================================================================

    /// Try to get the current animated value, returning None if lock is poisoned
    pub fn try_get(&self) -> Option<f32> {
        self.instance.read().ok().map(|g| g.get())
    }

    /// Try to get the current value as i32, returning None if lock is poisoned
    pub fn try_get_i32(&self) -> Option<i32> {
        self.instance.read().ok().map(|g| g.get_i32())
    }

    /// Try to get the current value as usize, returning None if lock is poisoned
    pub fn try_get_usize(&self) -> Option<usize> {
        self.instance.read().ok().map(|g| g.get_usize())
    }

    /// Try to start or resume the animation, returning false if lock is poisoned
    pub fn try_play(&self) -> bool {
        if let Ok(mut guard) = self.instance.write() {
            guard.play();
            self.trigger_render();
            true
        } else {
            false
        }
    }

    /// Try to pause the animation, returning false if lock is poisoned
    pub fn try_pause(&self) -> bool {
        if let Ok(mut guard) = self.instance.write() {
            guard.pause();
            true
        } else {
            false
        }
    }

    /// Try to reset the animation, returning false if lock is poisoned
    pub fn try_reset(&self) -> bool {
        if let Ok(mut guard) = self.instance.write() {
            guard.reset();
            self.trigger_render();
            true
        } else {
            false
        }
    }

    /// Try to check if the animation is running, returning None if lock is poisoned
    pub fn try_is_running(&self) -> Option<bool> {
        self.instance.read().ok().map(|g| g.is_running())
    }

    /// Try to check if the animation has completed, returning None if lock is poisoned
    pub fn try_is_completed(&self) -> Option<bool> {
        self.instance.read().ok().map(|g| g.is_completed())
    }

    /// Try to get the current animation state, returning None if lock is poisoned
    pub fn try_state(&self) -> Option<AnimationState> {
        self.instance.read().ok().map(|g| g.state())
    }

    /// Try to get the animation progress, returning None if lock is poisoned
    pub fn try_progress(&self) -> Option<f32> {
        self.instance.read().ok().map(|g| g.progress())
    }
}

impl std::fmt::Debug for AnimationHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let instance = self.instance.read().unwrap();
        f.debug_struct("AnimationHandle")
            .field("value", &instance.get())
            .field("state", &instance.state())
            .finish()
    }
}

/// Storage for animation hook
#[derive(Clone)]
struct AnimationStorage {
    handle: AnimationHandle,
}

fn new_animation_handle(
    animation: Animation,
    render_callback: Option<RenderCallback>,
) -> AnimationHandle {
    let instance = animation.start();
    AnimationHandle {
        instance: Arc::new(RwLock::new(instance)),
        last_tick: Arc::new(RwLock::new(Instant::now())),
        render_callback,
    }
}

/// Create an animation hook
///
/// Returns an `AnimationHandle` that can be used to control the animation
/// and get the current animated value.
///
/// # Example
///
/// ```ignore
/// use rnk::animation::{Animation, Easing, DurationExt};
///
/// fn my_component() -> Element {
///     let fade = use_animation(|| {
///         Animation::new()
///             .from(0.0)
///             .to(1.0)
///             .duration(300.ms())
///             .easing(Easing::EaseOut)
///     });
///
///     // Start animation on mount
///     use_effect_once(move || {
///         fade.play();
///     });
///
///     let opacity = fade.get();
///     // Use opacity in rendering...
/// }
/// ```
pub fn use_animation(init: impl FnOnce() -> Animation) -> AnimationHandle {
    let animation = init();
    let Some(ctx) = current_context() else {
        return new_animation_handle(animation, None);
    };
    let Ok(mut ctx_ref) = ctx.write() else {
        return new_animation_handle(animation, None);
    };
    let render_callback = ctx_ref.get_render_callback();
    let animation_for_hook = animation.clone();

    let storage = ctx_ref.use_hook(|| {
        AnimationStorage {
            handle: new_animation_handle(animation_for_hook, render_callback.clone()),
        }
    });

    storage
        .get::<AnimationStorage>()
        .map(|s| s.handle)
        .unwrap_or_else(|| new_animation_handle(animation, render_callback))
}

/// Create an animation that starts automatically
///
/// This is a convenience wrapper around `use_animation` that automatically
/// starts the animation when the component mounts.
///
/// # Example
///
/// ```ignore
/// let fade = use_animation_auto(|| Animation::fade_in(300.ms()));
/// let opacity = fade.get(); // Animation starts immediately
/// ```
pub fn use_animation_auto(init: impl FnOnce() -> Animation) -> AnimationHandle {
    let handle = use_animation(init);

    // Auto-start if idle
    if handle.state() == AnimationState::Idle {
        handle.play();
    }

    handle
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{DurationExt, Easing};
    use crate::hooks::context::{HookContext, with_hooks};

    #[test]
    fn test_animation_handle_basic() {
        let anim = Animation::new()
            .from(0.0)
            .to(100.0)
            .duration(100.ms())
            .easing(Easing::Linear);

        let handle = AnimationHandle {
            instance: Arc::new(RwLock::new(anim.start())),
            last_tick: Arc::new(RwLock::new(Instant::now())),
            render_callback: None,
        };

        assert_eq!(handle.state(), AnimationState::Idle);
        assert_eq!(handle.get(), 0.0);

        handle.play();
        assert!(handle.is_running());
    }

    #[test]
    fn test_use_animation_in_context() {
        let ctx = Arc::new(RwLock::new(HookContext::new()));

        let handle = with_hooks(ctx.clone(), || {
            use_animation(|| Animation::new().from(0.0).to(100.0).duration(100.ms()))
        });

        assert_eq!(handle.state(), AnimationState::Idle);
        handle.play();
        assert!(handle.is_running());
    }

    #[test]
    fn test_animation_persistence() {
        let ctx = Arc::new(RwLock::new(HookContext::new()));

        // First render - create animation
        let handle1 = with_hooks(ctx.clone(), || {
            use_animation(|| Animation::new().from(0.0).to(100.0).duration(100.ms()))
        });

        handle1.play();
        // Simulate some time passing
        handle1.instance.write().unwrap().tick(50.ms());

        // Second render - should get same animation instance
        let handle2 = with_hooks(ctx.clone(), || {
            use_animation(|| Animation::new().from(999.0).to(999.0).duration(999.ms()))
        });

        // Should still be running with progress
        assert!(handle2.is_running());
        assert!(handle2.get() > 0.0);
    }

    #[test]
    fn test_use_animation_without_context_does_not_panic() {
        let handle = use_animation(|| Animation::new().from(0.0).to(10.0).duration(100.ms()));
        assert_eq!(handle.state(), AnimationState::Idle);

        handle.play();
        assert!(handle.is_running());
    }
}
