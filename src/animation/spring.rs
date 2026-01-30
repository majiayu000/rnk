//! Spring animation system
//!
//! Provides physics-based spring animations inspired by Harmonica.
//! Springs can be used for smooth, natural-feeling animations.

/// Spring physics configuration
#[derive(Debug, Clone, Copy)]
pub struct Spring {
    /// Angular frequency (controls speed)
    angular_frequency: f32,
    /// Damping ratio (controls oscillation)
    damping_ratio: f32,
    /// Time delta per frame
    time_delta: f32,
}

impl Spring {
    /// Create a new spring with custom parameters
    ///
    /// # Arguments
    /// * `fps` - Target frames per second
    /// * `angular_frequency` - Controls animation speed (higher = faster)
    /// * `damping_ratio` - Controls oscillation behavior:
    ///   - < 1.0: Underdamped (bouncy, oscillates)
    ///   - = 1.0: Critically damped (fastest without oscillation)
    ///   - > 1.0: Overdamped (slow, no oscillation)
    pub fn new(fps: f32, angular_frequency: f32, damping_ratio: f32) -> Self {
        Self {
            angular_frequency,
            damping_ratio,
            time_delta: 1.0 / fps,
        }
    }

    /// Create a spring from time delta directly
    pub fn from_time_delta(time_delta: f32, angular_frequency: f32, damping_ratio: f32) -> Self {
        Self {
            angular_frequency,
            damping_ratio,
            time_delta,
        }
    }

    /// Create a smooth spring (critically damped)
    ///
    /// Best for UI elements that should move quickly without bouncing.
    pub fn smooth(fps: f32) -> Self {
        Self::new(fps, 6.0, 1.0)
    }

    /// Create a bouncy spring (underdamped)
    ///
    /// Good for playful animations with some overshoot.
    pub fn bouncy(fps: f32) -> Self {
        Self::new(fps, 6.0, 0.5)
    }

    /// Create a stiff spring (overdamped)
    ///
    /// For slow, deliberate movements without any bounce.
    pub fn stiff(fps: f32) -> Self {
        Self::new(fps, 4.0, 1.5)
    }

    /// Create a snappy spring
    ///
    /// Fast response with minimal overshoot.
    pub fn snappy(fps: f32) -> Self {
        Self::new(fps, 10.0, 0.9)
    }

    /// Create a gentle spring
    ///
    /// Slow, smooth movement.
    pub fn gentle(fps: f32) -> Self {
        Self::new(fps, 3.0, 1.0)
    }

    /// Update the spring simulation
    ///
    /// # Arguments
    /// * `position` - Current position
    /// * `velocity` - Current velocity
    /// * `target` - Target position
    ///
    /// # Returns
    /// Tuple of (new_position, new_velocity)
    pub fn update(&self, position: f32, velocity: f32, target: f32) -> (f32, f32) {
        let delta = position - target;
        let omega = self.angular_frequency;
        let zeta = self.damping_ratio;
        let dt = self.time_delta;

        // Spring physics calculation
        let acceleration = -2.0 * zeta * omega * velocity - omega * omega * delta;
        let new_velocity = velocity + acceleration * dt;
        let new_position = position + new_velocity * dt;

        (new_position, new_velocity)
    }

    /// Check if the spring has settled (reached target)
    ///
    /// # Arguments
    /// * `position` - Current position
    /// * `velocity` - Current velocity
    /// * `target` - Target position
    /// * `threshold` - Position threshold for "close enough"
    pub fn is_settled(&self, position: f32, velocity: f32, target: f32, threshold: f32) -> bool {
        let position_settled = (position - target).abs() < threshold;
        let velocity_settled = velocity.abs() < threshold;
        position_settled && velocity_settled
    }

    /// Get the angular frequency
    pub fn angular_frequency(&self) -> f32 {
        self.angular_frequency
    }

    /// Get the damping ratio
    pub fn damping_ratio(&self) -> f32 {
        self.damping_ratio
    }

    /// Get the time delta
    pub fn time_delta(&self) -> f32 {
        self.time_delta
    }
}

impl Default for Spring {
    fn default() -> Self {
        Self::smooth(60.0)
    }
}

/// Animated value using spring physics
#[derive(Debug, Clone)]
pub struct SpringValue {
    /// Current position
    position: f32,
    /// Current velocity
    velocity: f32,
    /// Target position
    target: f32,
    /// Spring configuration
    spring: Spring,
    /// Settlement threshold
    threshold: f32,
}

impl SpringValue {
    /// Create a new spring value
    pub fn new(initial: f32, spring: Spring) -> Self {
        Self {
            position: initial,
            velocity: 0.0,
            target: initial,
            spring,
            threshold: 0.01,
        }
    }

    /// Create with default smooth spring at 60 FPS
    pub fn smooth(initial: f32) -> Self {
        Self::new(initial, Spring::smooth(60.0))
    }

    /// Create with bouncy spring at 60 FPS
    pub fn bouncy(initial: f32) -> Self {
        Self::new(initial, Spring::bouncy(60.0))
    }

    /// Set the target value
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Set the target and reset velocity
    pub fn snap_to(&mut self, value: f32) {
        self.position = value;
        self.target = value;
        self.velocity = 0.0;
    }

    /// Update the animation (call each frame)
    pub fn tick(&mut self) {
        if !self.is_settled() {
            let (pos, vel) = self.spring.update(self.position, self.velocity, self.target);
            self.position = pos;
            self.velocity = vel;
        }
    }

    /// Get the current position
    pub fn get(&self) -> f32 {
        self.position
    }

    /// Get the current position as an integer
    pub fn get_i32(&self) -> i32 {
        self.position.round() as i32
    }

    /// Get the current position as usize (clamped to 0)
    pub fn get_usize(&self) -> usize {
        self.position.round().max(0.0) as usize
    }

    /// Get the target value
    pub fn target(&self) -> f32 {
        self.target
    }

    /// Get the current velocity
    pub fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Check if the animation has settled
    pub fn is_settled(&self) -> bool {
        self.spring
            .is_settled(self.position, self.velocity, self.target, self.threshold)
    }

    /// Set the settlement threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set the spring configuration
    pub fn with_spring(mut self, spring: Spring) -> Self {
        self.spring = spring;
        self
    }
}

/// 2D spring value for animating positions
#[derive(Debug, Clone)]
pub struct SpringValue2D {
    /// X component
    pub x: SpringValue,
    /// Y component
    pub y: SpringValue,
}

impl SpringValue2D {
    /// Create a new 2D spring value
    pub fn new(x: f32, y: f32, spring: Spring) -> Self {
        Self {
            x: SpringValue::new(x, spring),
            y: SpringValue::new(y, spring),
        }
    }

    /// Create with smooth spring
    pub fn smooth(x: f32, y: f32) -> Self {
        Self {
            x: SpringValue::smooth(x),
            y: SpringValue::smooth(y),
        }
    }

    /// Set the target position
    pub fn set_target(&mut self, x: f32, y: f32) {
        self.x.set_target(x);
        self.y.set_target(y);
    }

    /// Snap to a position
    pub fn snap_to(&mut self, x: f32, y: f32) {
        self.x.snap_to(x);
        self.y.snap_to(y);
    }

    /// Update the animation
    pub fn tick(&mut self) {
        self.x.tick();
        self.y.tick();
    }

    /// Get the current position
    pub fn get(&self) -> (f32, f32) {
        (self.x.get(), self.y.get())
    }

    /// Get the current position as integers
    pub fn get_i32(&self) -> (i32, i32) {
        (self.x.get_i32(), self.y.get_i32())
    }

    /// Check if settled
    pub fn is_settled(&self) -> bool {
        self.x.is_settled() && self.y.is_settled()
    }
}

/// Color spring for animating between colors
#[derive(Debug, Clone)]
pub struct SpringColor {
    /// Red component
    pub r: SpringValue,
    /// Green component
    pub g: SpringValue,
    /// Blue component
    pub b: SpringValue,
}

impl SpringColor {
    /// Create a new color spring
    pub fn new(r: u8, g: u8, b: u8, spring: Spring) -> Self {
        Self {
            r: SpringValue::new(r as f32, spring),
            g: SpringValue::new(g as f32, spring),
            b: SpringValue::new(b as f32, spring),
        }
    }

    /// Create with smooth spring
    pub fn smooth(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: SpringValue::smooth(r as f32),
            g: SpringValue::smooth(g as f32),
            b: SpringValue::smooth(b as f32),
        }
    }

    /// Set the target color
    pub fn set_target(&mut self, r: u8, g: u8, b: u8) {
        self.r.set_target(r as f32);
        self.g.set_target(g as f32);
        self.b.set_target(b as f32);
    }

    /// Snap to a color
    pub fn snap_to(&mut self, r: u8, g: u8, b: u8) {
        self.r.snap_to(r as f32);
        self.g.snap_to(g as f32);
        self.b.snap_to(b as f32);
    }

    /// Update the animation
    pub fn tick(&mut self) {
        self.r.tick();
        self.g.tick();
        self.b.tick();
    }

    /// Get the current color as RGB
    pub fn get(&self) -> (u8, u8, u8) {
        (
            self.r.get().clamp(0.0, 255.0) as u8,
            self.g.get().clamp(0.0, 255.0) as u8,
            self.b.get().clamp(0.0, 255.0) as u8,
        )
    }

    /// Check if settled
    pub fn is_settled(&self) -> bool {
        self.r.is_settled() && self.g.is_settled() && self.b.is_settled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spring_creation() {
        let spring = Spring::new(60.0, 6.0, 1.0);
        assert_eq!(spring.angular_frequency(), 6.0);
        assert_eq!(spring.damping_ratio(), 1.0);
        assert!((spring.time_delta() - 1.0 / 60.0).abs() < 0.0001);
    }

    #[test]
    fn test_spring_presets() {
        let smooth = Spring::smooth(60.0);
        assert_eq!(smooth.damping_ratio(), 1.0);

        let bouncy = Spring::bouncy(60.0);
        assert!(bouncy.damping_ratio() < 1.0);

        let stiff = Spring::stiff(60.0);
        assert!(stiff.damping_ratio() > 1.0);
    }

    #[test]
    fn test_spring_update() {
        let spring = Spring::smooth(60.0);
        let (pos, vel) = spring.update(0.0, 0.0, 100.0);

        // Should move towards target
        assert!(pos > 0.0);
        assert!(vel > 0.0);
    }

    #[test]
    fn test_spring_convergence() {
        let spring = Spring::smooth(60.0);
        let mut pos = 0.0;
        let mut vel = 0.0;
        let target = 100.0;

        // Simulate many frames
        for _ in 0..1000 {
            let (new_pos, new_vel) = spring.update(pos, vel, target);
            pos = new_pos;
            vel = new_vel;
        }

        // Should converge to target
        assert!((pos - target).abs() < 0.1);
        assert!(vel.abs() < 0.1);
    }

    #[test]
    fn test_spring_is_settled() {
        let spring = Spring::smooth(60.0);

        assert!(spring.is_settled(100.0, 0.0, 100.0, 0.01));
        assert!(!spring.is_settled(0.0, 0.0, 100.0, 0.01));
        assert!(!spring.is_settled(100.0, 1.0, 100.0, 0.01));
    }

    #[test]
    fn test_spring_value_creation() {
        let value = SpringValue::smooth(50.0);
        assert_eq!(value.get(), 50.0);
        assert_eq!(value.target(), 50.0);
        assert!(value.is_settled());
    }

    #[test]
    fn test_spring_value_animation() {
        let mut value = SpringValue::smooth(0.0);
        value.set_target(100.0);

        assert!(!value.is_settled());

        // Tick many times
        for _ in 0..1000 {
            value.tick();
        }

        assert!(value.is_settled());
        assert!((value.get() - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_spring_value_snap() {
        let mut value = SpringValue::smooth(0.0);
        value.set_target(100.0);
        value.tick();

        // Now snap to a different value
        value.snap_to(50.0);

        assert_eq!(value.get(), 50.0);
        assert_eq!(value.target(), 50.0);
        assert_eq!(value.velocity(), 0.0);
        assert!(value.is_settled());
    }

    #[test]
    fn test_spring_value_get_i32() {
        let value = SpringValue::smooth(42.7);
        assert_eq!(value.get_i32(), 43);
    }

    #[test]
    fn test_spring_value_get_usize() {
        let value = SpringValue::smooth(42.7);
        assert_eq!(value.get_usize(), 43);

        let negative = SpringValue::smooth(-10.0);
        assert_eq!(negative.get_usize(), 0);
    }

    #[test]
    fn test_spring_value_2d() {
        let mut value = SpringValue2D::smooth(0.0, 0.0);
        value.set_target(100.0, 50.0);

        for _ in 0..1000 {
            value.tick();
        }

        let (x, y) = value.get();
        assert!((x - 100.0).abs() < 0.1);
        assert!((y - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_spring_color() {
        let mut color = SpringColor::smooth(0, 0, 0);
        color.set_target(255, 128, 64);

        for _ in 0..1000 {
            color.tick();
        }

        let (r, g, b) = color.get();
        assert!((r as i32 - 255).abs() <= 1);
        assert!((g as i32 - 128).abs() <= 1);
        assert!((b as i32 - 64).abs() <= 1);
    }

    #[test]
    fn test_spring_default() {
        let spring = Spring::default();
        assert_eq!(spring.damping_ratio(), 1.0);
    }
}
