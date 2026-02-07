//! use_idle hook for detecting user inactivity
//!
//! Provides a way to detect when the user has been idle.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn app() -> Element {
//!     let is_idle = use_idle(Duration::from_secs(30));
//!
//!     if is_idle {
//!         Text::new("You've been idle for 30 seconds").into_element()
//!     } else {
//!         Text::new("Active").into_element()
//!     }
//! }
//! ```

use crate::hooks::use_interval::use_interval;
use crate::hooks::use_signal::use_signal;
use std::cell::RefCell;
use std::time::{Duration, Instant};

thread_local! {
    /// Last activity timestamp
    static LAST_ACTIVITY: RefCell<Instant> = RefCell::new(Instant::now());
}

const IDLE_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Record user activity
///
/// Call this when user input is detected to reset the idle timer.
pub fn record_activity() {
    LAST_ACTIVITY.with(|last| {
        *last.borrow_mut() = Instant::now();
    });
}

/// Get the duration since last activity
pub fn idle_duration() -> Duration {
    LAST_ACTIVITY.with(|last| last.borrow().elapsed())
}

/// Check if user has been idle for at least the given duration
pub fn is_idle(threshold: Duration) -> bool {
    idle_duration() >= threshold
}

/// Idle state information
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IdleState {
    /// Whether the user is currently idle
    pub is_idle: bool,
    /// Duration since last activity in seconds
    pub idle_seconds: u64,
}

impl IdleState {
    /// Create a new idle state
    pub fn new(threshold: Duration) -> Self {
        let duration = idle_duration();
        Self {
            is_idle: duration >= threshold,
            idle_seconds: duration.as_secs(),
        }
    }
}

/// Hook to check if user is idle
///
/// Returns true if the user has been idle for at least the given duration.
pub fn use_idle(threshold: Duration) -> bool {
    use_idle_refresh_tick();
    is_idle(threshold)
}

/// Hook to get detailed idle state
pub fn use_idle_state(threshold: Duration) -> IdleState {
    use_idle_refresh_tick();
    IdleState::new(threshold)
}

/// Hook to get idle duration in seconds
pub fn use_idle_seconds() -> u64 {
    use_idle_refresh_tick();
    idle_duration().as_secs()
}

fn use_idle_refresh_tick() {
    let tick = use_signal(|| 0u64);
    let tick_for_interval = tick.clone();
    use_interval(IDLE_POLL_INTERVAL, move || {
        tick_for_interval.update(|v| *v = v.wrapping_add(1));
    });
    let _ = tick.get();
}

/// Idle callback configuration
#[derive(Debug, Clone)]
pub struct IdleConfig {
    /// Threshold duration for idle detection
    pub threshold: Duration,
    /// Whether to trigger callback repeatedly while idle
    pub repeat: bool,
    /// Interval for repeated callbacks
    pub repeat_interval: Duration,
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self {
            threshold: Duration::from_secs(60),
            repeat: false,
            repeat_interval: Duration::from_secs(10),
        }
    }
}

impl IdleConfig {
    /// Create a new idle config
    pub fn new(threshold: Duration) -> Self {
        Self {
            threshold,
            ..Default::default()
        }
    }

    /// Set threshold
    pub fn threshold(mut self, threshold: Duration) -> Self {
        self.threshold = threshold;
        self
    }

    /// Enable repeated callbacks
    pub fn repeat(mut self, repeat: bool) -> Self {
        self.repeat = repeat;
        self
    }

    /// Set repeat interval
    pub fn repeat_interval(mut self, interval: Duration) -> Self {
        self.repeat_interval = interval;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_activity() {
        record_activity();
        let duration = idle_duration();
        assert!(duration < Duration::from_secs(1));
    }

    #[test]
    fn test_idle_duration() {
        record_activity();
        let duration = idle_duration();
        assert!(duration.as_millis() < 100);
    }

    #[test]
    fn test_is_idle() {
        record_activity();
        assert!(!is_idle(Duration::from_secs(1)));
        // Can't easily test true case without sleeping
    }

    #[test]
    fn test_idle_state() {
        record_activity();
        let state = IdleState::new(Duration::from_secs(60));
        assert!(!state.is_idle);
    }

    #[test]
    fn test_idle_state_default() {
        let state = IdleState::default();
        assert!(!state.is_idle);
        assert_eq!(state.idle_seconds, 0);
    }

    #[test]
    fn test_idle_config() {
        let config = IdleConfig::new(Duration::from_secs(30))
            .repeat(true)
            .repeat_interval(Duration::from_secs(5));

        assert_eq!(config.threshold, Duration::from_secs(30));
        assert!(config.repeat);
        assert_eq!(config.repeat_interval, Duration::from_secs(5));
    }

    #[test]
    fn test_use_idle_compiles() {
        fn _test() {
            let _ = use_idle(Duration::from_secs(30));
        }
    }

    #[test]
    fn test_use_idle_state_compiles() {
        fn _test() {
            let _ = use_idle_state(Duration::from_secs(30));
        }
    }

    #[test]
    fn test_use_idle_seconds_compiles() {
        fn _test() {
            let _ = use_idle_seconds();
        }
    }
}
