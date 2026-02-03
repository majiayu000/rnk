//! Keyframe animation system
//!
//! Provides time-based animations with keyframes and easing.

use super::easing::Easing;
use std::time::Duration;

/// Duration extension trait for ergonomic duration creation
pub trait DurationExt {
    /// Convert to Duration as milliseconds
    fn ms(self) -> Duration;
    /// Convert to Duration as seconds
    fn secs(self) -> Duration;
}

impl DurationExt for u64 {
    fn ms(self) -> Duration {
        Duration::from_millis(self)
    }

    fn secs(self) -> Duration {
        Duration::from_secs(self)
    }
}

impl DurationExt for f32 {
    fn ms(self) -> Duration {
        Duration::from_secs_f32(self / 1000.0)
    }

    fn secs(self) -> Duration {
        Duration::from_secs_f32(self)
    }
}

/// Animation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    /// Animation has not started
    Idle,
    /// Animation is running
    Running,
    /// Animation is paused
    Paused,
    /// Animation has completed
    Completed,
}

/// Animation direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationDirection {
    /// Play forward (from -> to)
    #[default]
    Normal,
    /// Play backward (to -> from)
    Reverse,
    /// Alternate between forward and backward
    Alternate,
    /// Alternate starting with backward
    AlternateReverse,
}

/// Animation fill mode (what value to show when not running)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillMode {
    /// Reset to initial value when done
    #[default]
    None,
    /// Keep the final value when done
    Forwards,
    /// Apply initial value before animation starts
    Backwards,
    /// Both forwards and backwards
    Both,
}

/// A keyframe animation configuration
#[derive(Debug, Clone)]
pub struct Animation {
    /// Starting value
    from: f32,
    /// Ending value
    to: f32,
    /// Animation duration
    duration: Duration,
    /// Easing function
    easing: Easing,
    /// Number of iterations (0 = infinite)
    iterations: u32,
    /// Animation direction
    direction: AnimationDirection,
    /// Delay before starting
    delay: Duration,
    /// Fill mode
    fill_mode: FillMode,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            from: 0.0,
            to: 1.0,
            duration: Duration::from_millis(300),
            easing: Easing::default(),
            iterations: 1,
            direction: AnimationDirection::default(),
            delay: Duration::ZERO,
            fill_mode: FillMode::default(),
        }
    }
}

impl Animation {
    /// Create a new animation builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Create animation with keyframes builder
    pub fn keyframes() -> Self {
        Self::default()
    }

    /// Set the starting value
    pub fn from(mut self, value: f32) -> Self {
        self.from = value;
        self
    }

    /// Set the ending value
    pub fn to(mut self, value: f32) -> Self {
        self.to = value;
        self
    }

    /// Set both from and to values
    pub fn values(mut self, from: f32, to: f32) -> Self {
        self.from = from;
        self.to = to;
        self
    }

    /// Set the duration
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set the easing function
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Set the number of iterations (0 = infinite)
    pub fn iterations(mut self, count: u32) -> Self {
        self.iterations = count;
        self
    }

    /// Set to loop infinitely
    pub fn infinite(mut self) -> Self {
        self.iterations = 0;
        self
    }

    /// Set the animation direction
    pub fn direction(mut self, direction: AnimationDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set the delay before starting
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Set the fill mode
    pub fn fill_mode(mut self, mode: FillMode) -> Self {
        self.fill_mode = mode;
        self
    }

    /// Get the from value
    pub fn get_from(&self) -> f32 {
        self.from
    }

    /// Get the to value
    pub fn get_to(&self) -> f32 {
        self.to
    }

    /// Get the duration
    pub fn get_duration(&self) -> Duration {
        self.duration
    }

    /// Get the easing
    pub fn get_easing(&self) -> Easing {
        self.easing
    }

    /// Get iterations
    pub fn get_iterations(&self) -> u32 {
        self.iterations
    }

    /// Get delay
    pub fn get_delay(&self) -> Duration {
        self.delay
    }

    /// Create a running animation instance
    pub fn start(&self) -> AnimationInstance {
        AnimationInstance::new(self.clone())
    }
}

/// A running animation instance
#[derive(Debug, Clone)]
pub struct AnimationInstance {
    /// Animation configuration
    config: Animation,
    /// Current state
    state: AnimationState,
    /// Elapsed time
    elapsed: Duration,
    /// Current iteration
    current_iteration: u32,
    /// Current value
    value: f32,
}

impl AnimationInstance {
    /// Create a new animation instance
    pub fn new(config: Animation) -> Self {
        let initial_value = match config.fill_mode {
            FillMode::Backwards | FillMode::Both => config.from,
            _ => config.from,
        };

        Self {
            config,
            state: AnimationState::Idle,
            elapsed: Duration::ZERO,
            current_iteration: 0,
            value: initial_value,
        }
    }

    /// Start or resume the animation
    pub fn play(&mut self) {
        match self.state {
            AnimationState::Idle | AnimationState::Paused => {
                self.state = AnimationState::Running;
            }
            _ => {}
        }
    }

    /// Pause the animation
    pub fn pause(&mut self) {
        if self.state == AnimationState::Running {
            self.state = AnimationState::Paused;
        }
    }

    /// Reset the animation to the beginning
    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
        self.current_iteration = 0;
        self.state = AnimationState::Idle;
        self.value = self.config.from;
    }

    /// Update the animation with elapsed time (call each frame)
    pub fn tick(&mut self, delta: Duration) {
        if self.state != AnimationState::Running {
            return;
        }

        self.elapsed += delta;

        // Handle delay
        if self.elapsed < self.config.delay {
            return;
        }

        let active_elapsed = self.elapsed - self.config.delay;
        let duration_ms = self.config.duration.as_secs_f32() * 1000.0;

        if duration_ms <= 0.0 {
            self.value = self.config.to;
            self.state = AnimationState::Completed;
            return;
        }

        // Calculate progress within current iteration
        let total_progress_ms = active_elapsed.as_secs_f32() * 1000.0;
        let iteration_progress = total_progress_ms / duration_ms;
        let current_iteration = iteration_progress.floor() as u32;

        // Check if we've completed all iterations
        if self.config.iterations > 0 && current_iteration >= self.config.iterations {
            self.current_iteration = self.config.iterations;
            self.state = AnimationState::Completed;

            // Set final value based on fill mode
            self.value = match self.config.fill_mode {
                FillMode::Forwards | FillMode::Both => self.config.to,
                _ => self.config.from,
            };
            return;
        }

        self.current_iteration = current_iteration;

        // Calculate t within current iteration [0, 1]
        let t = iteration_progress.fract();

        // Apply direction
        let effective_t = match self.config.direction {
            AnimationDirection::Normal => t,
            AnimationDirection::Reverse => 1.0 - t,
            AnimationDirection::Alternate => {
                if current_iteration % 2 == 0 {
                    t
                } else {
                    1.0 - t
                }
            }
            AnimationDirection::AlternateReverse => {
                if current_iteration % 2 == 0 {
                    1.0 - t
                } else {
                    t
                }
            }
        };

        // Apply easing and interpolate
        self.value = self
            .config
            .easing
            .interpolate(self.config.from, self.config.to, effective_t);
    }

    /// Get the current value
    pub fn get(&self) -> f32 {
        self.value
    }

    /// Get the current value as i32
    pub fn get_i32(&self) -> i32 {
        self.value.round() as i32
    }

    /// Get the current value as usize (clamped to 0)
    pub fn get_usize(&self) -> usize {
        self.value.round().max(0.0) as usize
    }

    /// Get the current state
    pub fn state(&self) -> AnimationState {
        self.state
    }

    /// Check if the animation is running
    pub fn is_running(&self) -> bool {
        self.state == AnimationState::Running
    }

    /// Check if the animation has completed
    pub fn is_completed(&self) -> bool {
        self.state == AnimationState::Completed
    }

    /// Get the current iteration
    pub fn current_iteration(&self) -> u32 {
        self.current_iteration
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Get progress as a value from 0.0 to 1.0
    pub fn progress(&self) -> f32 {
        if self.config.duration.is_zero() {
            return 1.0;
        }

        let active_elapsed = self.elapsed.saturating_sub(self.config.delay);
        let progress = active_elapsed.as_secs_f32() / self.config.duration.as_secs_f32();

        if self.config.iterations == 0 {
            progress.fract()
        } else {
            (progress / self.config.iterations as f32).min(1.0)
        }
    }
}

/// Preset animations
impl Animation {
    /// Fade in animation (0 to 1)
    pub fn fade_in(duration: Duration) -> Self {
        Self::new()
            .from(0.0)
            .to(1.0)
            .duration(duration)
            .easing(Easing::EaseOut)
    }

    /// Fade out animation (1 to 0)
    pub fn fade_out(duration: Duration) -> Self {
        Self::new()
            .from(1.0)
            .to(0.0)
            .duration(duration)
            .easing(Easing::EaseIn)
    }

    /// Slide animation
    pub fn slide(from: f32, to: f32, duration: Duration) -> Self {
        Self::new()
            .from(from)
            .to(to)
            .duration(duration)
            .easing(Easing::EaseInOutCubic)
    }

    /// Bounce animation
    pub fn bounce(from: f32, to: f32, duration: Duration) -> Self {
        Self::new()
            .from(from)
            .to(to)
            .duration(duration)
            .easing(Easing::EaseOutBounce)
    }

    /// Pulse animation (grows and shrinks)
    pub fn pulse(duration: Duration) -> Self {
        Self::new()
            .from(1.0)
            .to(1.2)
            .duration(duration)
            .direction(AnimationDirection::Alternate)
            .infinite()
    }

    /// Blink animation
    pub fn blink(duration: Duration) -> Self {
        Self::new()
            .from(0.0)
            .to(1.0)
            .duration(duration)
            .direction(AnimationDirection::Alternate)
            .infinite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_builder() {
        let anim = Animation::new()
            .from(0.0)
            .to(100.0)
            .duration(300.ms())
            .easing(Easing::EaseOut);

        assert_eq!(anim.get_from(), 0.0);
        assert_eq!(anim.get_to(), 100.0);
        assert_eq!(anim.get_duration(), Duration::from_millis(300));
    }

    #[test]
    fn test_duration_ext() {
        assert_eq!(300u64.ms(), Duration::from_millis(300));
        assert_eq!(2u64.secs(), Duration::from_secs(2));
    }

    #[test]
    fn test_animation_instance_start() {
        let anim = Animation::new().from(0.0).to(100.0).duration(100.ms());

        let mut instance = anim.start();
        assert_eq!(instance.state(), AnimationState::Idle);

        instance.play();
        assert_eq!(instance.state(), AnimationState::Running);
    }

    #[test]
    fn test_animation_tick() {
        let anim = Animation::new()
            .from(0.0)
            .to(100.0)
            .duration(100.ms())
            .easing(Easing::Linear)
            .fill_mode(FillMode::Forwards);

        let mut instance = anim.start();
        instance.play();

        // At t=0
        assert_eq!(instance.get(), 0.0);

        // At t=50ms (halfway)
        instance.tick(50.ms());
        assert!((instance.get() - 50.0).abs() < 1.0);

        // At t=100ms (complete) - with FillMode::Forwards, should stay at 100
        instance.tick(50.ms());
        assert!(instance.is_completed());
        assert!((instance.get() - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_animation_completion() {
        let anim = Animation::new()
            .from(0.0)
            .to(100.0)
            .duration(100.ms())
            .iterations(1);

        let mut instance = anim.start();
        instance.play();

        instance.tick(150.ms());
        assert!(instance.is_completed());
    }

    #[test]
    fn test_animation_infinite() {
        let anim = Animation::new()
            .from(0.0)
            .to(100.0)
            .duration(100.ms())
            .infinite();

        let mut instance = anim.start();
        instance.play();

        // Should still be running after many iterations
        instance.tick(500.ms());
        assert!(instance.is_running());
    }

    #[test]
    fn test_animation_alternate() {
        let anim = Animation::new()
            .from(0.0)
            .to(100.0)
            .duration(100.ms())
            .direction(AnimationDirection::Alternate)
            .iterations(2);

        let mut instance = anim.start();
        instance.play();

        // First iteration: forward
        instance.tick(50.ms());
        let v1 = instance.get();
        assert!(v1 > 0.0 && v1 < 100.0);

        // Second iteration: backward
        instance.tick(100.ms());
        let v2 = instance.get();
        assert!(v2 > 0.0 && v2 < 100.0);
    }

    #[test]
    fn test_animation_delay() {
        let anim = Animation::new()
            .from(0.0)
            .to(100.0)
            .duration(100.ms())
            .delay(50.ms())
            .easing(Easing::Linear);

        let mut instance = anim.start();
        instance.play();

        // During delay, value should stay at from
        instance.tick(25.ms());
        assert_eq!(instance.get(), 0.0);

        // After delay, animation starts
        instance.tick(75.ms()); // 25 + 75 = 100ms total, 50ms into animation
        assert!((instance.get() - 50.0).abs() < 1.0);
    }

    #[test]
    fn test_animation_pause_resume() {
        let anim = Animation::new()
            .from(0.0)
            .to(100.0)
            .duration(100.ms())
            .easing(Easing::Linear);

        let mut instance = anim.start();
        instance.play();
        instance.tick(50.ms());
        let v1 = instance.get();

        instance.pause();
        instance.tick(50.ms()); // Should not advance
        assert_eq!(instance.get(), v1);

        instance.play();
        instance.tick(25.ms());
        assert!(instance.get() > v1);
    }

    #[test]
    fn test_animation_reset() {
        let anim = Animation::new().from(0.0).to(100.0).duration(100.ms());

        let mut instance = anim.start();
        instance.play();
        instance.tick(50.ms());

        instance.reset();
        assert_eq!(instance.state(), AnimationState::Idle);
        assert_eq!(instance.get(), 0.0);
        assert_eq!(instance.elapsed(), Duration::ZERO);
    }

    #[test]
    fn test_preset_fade_in() {
        let anim = Animation::fade_in(200.ms());
        assert_eq!(anim.get_from(), 0.0);
        assert_eq!(anim.get_to(), 1.0);
    }

    #[test]
    fn test_preset_pulse() {
        let anim = Animation::pulse(500.ms());
        assert_eq!(anim.get_iterations(), 0); // infinite
    }
}
