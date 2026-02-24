//! Transition hook for smooth value changes
//!
//! Provides a simple hook for transitioning between values with easing.

use crate::animation::{Animation, AnimationInstance, Easing, FillMode};
use crate::hooks::context::{RenderCallback, current_context};
use crate::hooks::lock_utils::{read_or_recover, write_or_recover};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Handle for a transitioning value
#[derive(Clone)]
pub struct TransitionHandle {
    current: Arc<RwLock<f32>>,
    target: Arc<RwLock<f32>>,
    instance: Arc<RwLock<Option<AnimationInstance>>>,
    duration: Duration,
    easing: Easing,
    last_tick: Arc<RwLock<Instant>>,
    render_callback: Option<RenderCallback>,
}

impl TransitionHandle {
    /// Get the current value
    pub fn get(&self) -> f32 {
        if let Some(ref instance) = *read_or_recover(&self.instance) {
            if instance.is_running() {
                return instance.get();
            }
        }
        *read_or_recover(&self.current)
    }

    /// Get the current value as i32
    pub fn get_i32(&self) -> i32 {
        self.get().round() as i32
    }

    /// Get the current value as usize
    pub fn get_usize(&self) -> usize {
        self.get().round().max(0.0) as usize
    }

    /// Get the target value
    pub fn target(&self) -> f32 {
        *read_or_recover(&self.target)
    }

    /// Set a new target value and start transitioning
    pub fn set(&self, value: f32) {
        let current = self.get();
        *write_or_recover(&self.target) = value;

        if (current - value).abs() < 0.001 {
            // Already at target, no transition needed
            *write_or_recover(&self.current) = value;
            *write_or_recover(&self.instance) = None;
            return;
        }

        // Create new animation from current to target
        let anim = Animation::new()
            .from(current)
            .to(value)
            .duration(self.duration)
            .easing(self.easing)
            .fill_mode(FillMode::Forwards);

        let mut instance = anim.start();
        instance.play();

        *write_or_recover(&self.instance) = Some(instance);
        *write_or_recover(&self.last_tick) = Instant::now();

        self.trigger_render();
    }

    /// Set value immediately without transition
    pub fn set_immediate(&self, value: f32) {
        *write_or_recover(&self.current) = value;
        *write_or_recover(&self.target) = value;
        *write_or_recover(&self.instance) = None;
        self.trigger_render();
    }

    /// Check if currently transitioning
    pub fn is_transitioning(&self) -> bool {
        read_or_recover(&self.instance)
            .as_ref()
            .is_some_and(|i| i.is_running())
    }

    /// Tick the transition (called internally)
    pub fn tick(&self) {
        let now = Instant::now();
        let delta = {
            let mut last = write_or_recover(&self.last_tick);
            let delta = now.duration_since(*last);
            *last = now;
            delta
        };

        let mut instance_guard = write_or_recover(&self.instance);
        if let Some(ref mut instance) = *instance_guard {
            let was_running = instance.is_running();
            instance.tick(delta);

            if instance.is_completed() {
                // Update current to final value
                *write_or_recover(&self.current) = *read_or_recover(&self.target);
                *instance_guard = None;
            } else if was_running && instance.is_running() {
                drop(instance_guard);
                self.trigger_render();
            }
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

    /// Try to get the current value, returning None if lock is poisoned
    pub fn try_get(&self) -> Option<f32> {
        let instance_guard = self.instance.read().ok()?;
        if let Some(ref instance) = *instance_guard {
            if instance.is_running() {
                return Some(instance.get());
            }
        }
        self.current.read().ok().map(|g| *g)
    }

    impl_try_get_conversions!();

    /// Try to get the target value, returning None if lock is poisoned
    pub fn try_target(&self) -> Option<f32> {
        self.target.read().ok().map(|g| *g)
    }

    /// Try to set a new target value, returning false if lock is poisoned
    pub fn try_set(&self, value: f32) -> bool {
        let current = match self.try_get() {
            Some(v) => v,
            None => return false,
        };

        if self.target.write().ok().map(|mut g| *g = value).is_none() {
            return false;
        }

        if (current - value).abs() < 0.001 {
            // Already at target, no transition needed
            if self.current.write().ok().map(|mut g| *g = value).is_none() {
                return false;
            }
            if self.instance.write().ok().map(|mut g| *g = None).is_none() {
                return false;
            }
            return true;
        }

        // Create new animation from current to target
        let anim = Animation::new()
            .from(current)
            .to(value)
            .duration(self.duration)
            .easing(self.easing)
            .fill_mode(FillMode::Forwards);

        let mut instance = anim.start();
        instance.play();

        if self
            .instance
            .write()
            .ok()
            .map(|mut g| *g = Some(instance))
            .is_none()
        {
            return false;
        }
        if self
            .last_tick
            .write()
            .ok()
            .map(|mut g| *g = Instant::now())
            .is_none()
        {
            return false;
        }

        self.trigger_render();
        true
    }

    /// Try to set value immediately without transition, returning false if lock is poisoned
    pub fn try_set_immediate(&self, value: f32) -> bool {
        if self.current.write().ok().map(|mut g| *g = value).is_none() {
            return false;
        }
        if self.target.write().ok().map(|mut g| *g = value).is_none() {
            return false;
        }
        if self.instance.write().ok().map(|mut g| *g = None).is_none() {
            return false;
        }
        self.trigger_render();
        true
    }

    /// Try to check if currently transitioning, returning None if lock is poisoned
    pub fn try_is_transitioning(&self) -> Option<bool> {
        self.instance
            .read()
            .ok()
            .map(|g| g.as_ref().is_some_and(|i| i.is_running()))
    }
}

impl std::fmt::Debug for TransitionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionHandle")
            .field("current", &self.get())
            .field("target", &self.target())
            .field("transitioning", &self.is_transitioning())
            .finish()
    }
}

/// Storage for transition hook
#[derive(Clone)]
struct TransitionStorage {
    handle: TransitionHandle,
}

fn new_transition_handle(
    initial: f32,
    duration: Duration,
    easing: Easing,
    render_callback: Option<RenderCallback>,
) -> TransitionHandle {
    TransitionHandle {
        current: Arc::new(RwLock::new(initial)),
        target: Arc::new(RwLock::new(initial)),
        instance: Arc::new(RwLock::new(None)),
        duration,
        easing,
        last_tick: Arc::new(RwLock::new(Instant::now())),
        render_callback,
    }
}

/// Create a transition hook for smooth value changes
///
/// Returns a tuple of (current_value, set_function) where setting a new value
/// will smoothly transition from the current value.
///
/// # Example
///
/// ```ignore
/// use rnk::animation::DurationExt;
///
/// fn my_component() -> Element {
///     let position = use_transition(0.0, 200.ms());
///
///     use_input(move |input, _| {
///         if input == "j" {
///             position.set(position.target() + 10.0);
///         }
///     });
///
///     let y = position.get_i32();
///     // Use y for positioning...
/// }
/// ```
pub fn use_transition(initial: f32, duration: Duration) -> TransitionHandle {
    use_transition_with_easing(initial, duration, Easing::EaseInOut)
}

/// Create a transition hook with custom easing
///
/// # Example
///
/// ```ignore
/// use rnk::animation::{DurationExt, Easing};
///
/// let scale = use_transition_with_easing(1.0, 150.ms(), Easing::EaseOutBack);
/// ```
pub fn use_transition_with_easing(
    initial: f32,
    duration: Duration,
    easing: Easing,
) -> TransitionHandle {
    let Some(ctx) = current_context() else {
        return new_transition_handle(initial, duration, easing, None);
    };
    let Ok(mut ctx_ref) = ctx.try_borrow_mut() else {
        return new_transition_handle(initial, duration, easing, None);
    };

    let render_callback = ctx_ref.get_render_callback();

    let storage = ctx_ref.use_hook(|| TransitionStorage {
        handle: new_transition_handle(initial, duration, easing, render_callback.clone()),
    });

    storage
        .get::<TransitionStorage>()
        .map(|s| s.handle)
        .unwrap_or_else(|| new_transition_handle(initial, duration, easing, render_callback))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::DurationExt;
    use crate::hooks::context::{HookContext, with_hooks};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_transition_handle_basic() {
        let handle = TransitionHandle {
            current: Arc::new(RwLock::new(0.0)),
            target: Arc::new(RwLock::new(0.0)),
            instance: Arc::new(RwLock::new(None)),
            duration: Duration::from_millis(100),
            easing: Easing::Linear,
            last_tick: Arc::new(RwLock::new(Instant::now())),
            render_callback: None,
        };

        assert_eq!(handle.get(), 0.0);
        assert!(!handle.is_transitioning());
    }

    #[test]
    fn test_transition_set() {
        let handle = TransitionHandle {
            current: Arc::new(RwLock::new(0.0)),
            target: Arc::new(RwLock::new(0.0)),
            instance: Arc::new(RwLock::new(None)),
            duration: Duration::from_millis(100),
            easing: Easing::Linear,
            last_tick: Arc::new(RwLock::new(Instant::now())),
            render_callback: None,
        };

        handle.set(100.0);
        assert!(handle.is_transitioning());
        assert_eq!(handle.target(), 100.0);
    }

    #[test]
    fn test_transition_immediate() {
        let handle = TransitionHandle {
            current: Arc::new(RwLock::new(0.0)),
            target: Arc::new(RwLock::new(0.0)),
            instance: Arc::new(RwLock::new(None)),
            duration: Duration::from_millis(100),
            easing: Easing::Linear,
            last_tick: Arc::new(RwLock::new(Instant::now())),
            render_callback: None,
        };

        handle.set_immediate(50.0);
        assert!(!handle.is_transitioning());
        assert_eq!(handle.get(), 50.0);
        assert_eq!(handle.target(), 50.0);
    }

    #[test]
    fn test_use_transition_in_context() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        let handle = with_hooks(ctx.clone(), || use_transition(0.0, 100.ms()));

        assert_eq!(handle.get(), 0.0);
        handle.set(100.0);
        assert!(handle.is_transitioning());
    }

    #[test]
    fn test_transition_persistence() {
        let ctx = Rc::new(RefCell::new(HookContext::new()));

        // First render
        let handle1 = with_hooks(ctx.clone(), || use_transition(0.0, 100.ms()));
        handle1.set(50.0);

        // Second render - should preserve state
        let handle2 = with_hooks(ctx.clone(), || use_transition(999.0, 999.ms()));

        assert_eq!(handle2.target(), 50.0);
    }

    #[test]
    fn test_transition_no_change() {
        let handle = TransitionHandle {
            current: Arc::new(RwLock::new(50.0)),
            target: Arc::new(RwLock::new(50.0)),
            instance: Arc::new(RwLock::new(None)),
            duration: Duration::from_millis(100),
            easing: Easing::Linear,
            last_tick: Arc::new(RwLock::new(Instant::now())),
            render_callback: None,
        };

        // Setting to same value should not start transition
        handle.set(50.0);
        assert!(!handle.is_transitioning());
    }

    #[test]
    fn test_use_transition_without_context_does_not_panic() {
        let handle = use_transition(0.0, 100.ms());
        assert_eq!(handle.get(), 0.0);

        handle.set(100.0);
        assert!(handle.is_transitioning());
        assert_eq!(handle.target(), 100.0);
    }
}
