//! Easing functions for animations
//!
//! Provides standard easing functions for smooth animations.
//! Based on Robert Penner's easing equations.

use std::f32::consts::PI;

/// Easing function type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Easing {
    /// Linear interpolation (no easing)
    #[default]
    Linear,
    /// Ease in (slow start)
    EaseIn,
    /// Ease out (slow end)
    EaseOut,
    /// Ease in and out (slow start and end)
    EaseInOut,
    /// Quadratic ease in
    EaseInQuad,
    /// Quadratic ease out
    EaseOutQuad,
    /// Quadratic ease in and out
    EaseInOutQuad,
    /// Cubic ease in
    EaseInCubic,
    /// Cubic ease out
    EaseOutCubic,
    /// Cubic ease in and out
    EaseInOutCubic,
    /// Quartic ease in
    EaseInQuart,
    /// Quartic ease out
    EaseOutQuart,
    /// Quartic ease in and out
    EaseInOutQuart,
    /// Sine ease in
    EaseInSine,
    /// Sine ease out
    EaseOutSine,
    /// Sine ease in and out
    EaseInOutSine,
    /// Exponential ease in
    EaseInExpo,
    /// Exponential ease out
    EaseOutExpo,
    /// Exponential ease in and out
    EaseInOutExpo,
    /// Circular ease in
    EaseInCirc,
    /// Circular ease out
    EaseOutCirc,
    /// Circular ease in and out
    EaseInOutCirc,
    /// Back ease in (overshoots then returns)
    EaseInBack,
    /// Back ease out
    EaseOutBack,
    /// Back ease in and out
    EaseInOutBack,
    /// Elastic ease in (bouncy)
    EaseInElastic,
    /// Elastic ease out
    EaseOutElastic,
    /// Elastic ease in and out
    EaseInOutElastic,
    /// Bounce ease in
    EaseInBounce,
    /// Bounce ease out
    EaseOutBounce,
    /// Bounce ease in and out
    EaseInOutBounce,
}

impl Easing {
    /// Apply the easing function to a value t in [0, 1]
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);

        match self {
            Easing::Linear => t,
            Easing::EaseIn | Easing::EaseInQuad => ease_in_quad(t),
            Easing::EaseOut | Easing::EaseOutQuad => ease_out_quad(t),
            Easing::EaseInOut | Easing::EaseInOutQuad => ease_in_out_quad(t),
            Easing::EaseInCubic => ease_in_cubic(t),
            Easing::EaseOutCubic => ease_out_cubic(t),
            Easing::EaseInOutCubic => ease_in_out_cubic(t),
            Easing::EaseInQuart => ease_in_quart(t),
            Easing::EaseOutQuart => ease_out_quart(t),
            Easing::EaseInOutQuart => ease_in_out_quart(t),
            Easing::EaseInSine => ease_in_sine(t),
            Easing::EaseOutSine => ease_out_sine(t),
            Easing::EaseInOutSine => ease_in_out_sine(t),
            Easing::EaseInExpo => ease_in_expo(t),
            Easing::EaseOutExpo => ease_out_expo(t),
            Easing::EaseInOutExpo => ease_in_out_expo(t),
            Easing::EaseInCirc => ease_in_circ(t),
            Easing::EaseOutCirc => ease_out_circ(t),
            Easing::EaseInOutCirc => ease_in_out_circ(t),
            Easing::EaseInBack => ease_in_back(t),
            Easing::EaseOutBack => ease_out_back(t),
            Easing::EaseInOutBack => ease_in_out_back(t),
            Easing::EaseInElastic => ease_in_elastic(t),
            Easing::EaseOutElastic => ease_out_elastic(t),
            Easing::EaseInOutElastic => ease_in_out_elastic(t),
            Easing::EaseInBounce => ease_in_bounce(t),
            Easing::EaseOutBounce => ease_out_bounce(t),
            Easing::EaseInOutBounce => ease_in_out_bounce(t),
        }
    }

    /// Interpolate between two values using this easing
    pub fn interpolate(&self, from: f32, to: f32, t: f32) -> f32 {
        let eased = self.apply(t);
        from + (to - from) * eased
    }
}

// Quadratic
fn ease_in_quad(t: f32) -> f32 {
    t * t
}

fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

// Cubic
fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

// Quartic
fn ease_in_quart(t: f32) -> f32 {
    t * t * t * t
}

fn ease_out_quart(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(4)
}

fn ease_in_out_quart(t: f32) -> f32 {
    if t < 0.5 {
        8.0 * t * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
    }
}

// Sine
fn ease_in_sine(t: f32) -> f32 {
    1.0 - (t * PI / 2.0).cos()
}

fn ease_out_sine(t: f32) -> f32 {
    (t * PI / 2.0).sin()
}

fn ease_in_out_sine(t: f32) -> f32 {
    -(PI * t).cos() / 2.0 + 0.5
}

// Exponential
fn ease_in_expo(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else {
        2.0_f32.powf(10.0 * t - 10.0)
    }
}

fn ease_out_expo(t: f32) -> f32 {
    if t == 1.0 {
        1.0
    } else {
        1.0 - 2.0_f32.powf(-10.0 * t)
    }
}

fn ease_in_out_expo(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else if t < 0.5 {
        2.0_f32.powf(20.0 * t - 10.0) / 2.0
    } else {
        (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) / 2.0
    }
}

// Circular
fn ease_in_circ(t: f32) -> f32 {
    1.0 - (1.0 - t * t).sqrt()
}

fn ease_out_circ(t: f32) -> f32 {
    (1.0 - (t - 1.0).powi(2)).sqrt()
}

fn ease_in_out_circ(t: f32) -> f32 {
    if t < 0.5 {
        (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
    } else {
        ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
    }
}

// Back (overshoot)
const C1: f32 = 1.70158;
const C2: f32 = C1 * 1.525;
const C3: f32 = C1 + 1.0;

fn ease_in_back(t: f32) -> f32 {
    C3 * t * t * t - C1 * t * t
}

fn ease_out_back(t: f32) -> f32 {
    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

fn ease_in_out_back(t: f32) -> f32 {
    if t < 0.5 {
        ((2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2)) / 2.0
    } else {
        ((2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (t * 2.0 - 2.0) + C2) + 2.0) / 2.0
    }
}

// Elastic
const C4: f32 = (2.0 * PI) / 3.0;
const C5: f32 = (2.0 * PI) / 4.5;

fn ease_in_elastic(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        -2.0_f32.powf(10.0 * t - 10.0) * ((t * 10.0 - 10.75) * C4).sin()
    }
}

fn ease_out_elastic(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * C4).sin() + 1.0
    }
}

fn ease_in_out_elastic(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else if t < 0.5 {
        -(2.0_f32.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * C5).sin()) / 2.0
    } else {
        (2.0_f32.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * C5).sin()) / 2.0 + 1.0
    }
}

// Bounce
fn ease_out_bounce(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;

    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984375
    }
}

fn ease_in_bounce(t: f32) -> f32 {
    1.0 - ease_out_bounce(1.0 - t)
}

fn ease_in_out_bounce(t: f32) -> f32 {
    if t < 0.5 {
        (1.0 - ease_out_bounce(1.0 - 2.0 * t)) / 2.0
    } else {
        (1.0 + ease_out_bounce(2.0 * t - 1.0)) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear() {
        let easing = Easing::Linear;
        assert_eq!(easing.apply(0.0), 0.0);
        assert_eq!(easing.apply(0.5), 0.5);
        assert_eq!(easing.apply(1.0), 1.0);
    }

    #[test]
    fn test_ease_in_out_boundaries() {
        for easing in [
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
            Easing::EaseInCubic,
            Easing::EaseOutCubic,
            Easing::EaseInOutCubic,
        ] {
            assert!((easing.apply(0.0) - 0.0).abs() < 0.001, "{:?} at 0", easing);
            assert!((easing.apply(1.0) - 1.0).abs() < 0.001, "{:?} at 1", easing);
        }
    }

    #[test]
    fn test_interpolate() {
        let easing = Easing::Linear;
        assert_eq!(easing.interpolate(0.0, 100.0, 0.0), 0.0);
        assert_eq!(easing.interpolate(0.0, 100.0, 0.5), 50.0);
        assert_eq!(easing.interpolate(0.0, 100.0, 1.0), 100.0);
    }

    #[test]
    fn test_clamp() {
        let easing = Easing::Linear;
        assert_eq!(easing.apply(-0.5), 0.0);
        assert_eq!(easing.apply(1.5), 1.0);
    }

    #[test]
    fn test_ease_in_slower_start() {
        let ease_in = Easing::EaseInQuad;
        // At t=0.25, ease in should be less than linear
        assert!(ease_in.apply(0.25) < 0.25);
    }

    #[test]
    fn test_ease_out_faster_start() {
        let ease_out = Easing::EaseOutQuad;
        // At t=0.25, ease out should be greater than linear
        assert!(ease_out.apply(0.25) > 0.25);
    }

    #[test]
    fn test_bounce_boundaries() {
        assert!((Easing::EaseOutBounce.apply(0.0) - 0.0).abs() < 0.001);
        assert!((Easing::EaseOutBounce.apply(1.0) - 1.0).abs() < 0.001);
        assert!((Easing::EaseInBounce.apply(0.0) - 0.0).abs() < 0.001);
        assert!((Easing::EaseInBounce.apply(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_elastic_boundaries() {
        assert!((Easing::EaseOutElastic.apply(0.0) - 0.0).abs() < 0.001);
        assert!((Easing::EaseOutElastic.apply(1.0) - 1.0).abs() < 0.001);
        assert!((Easing::EaseInElastic.apply(0.0) - 0.0).abs() < 0.001);
        assert!((Easing::EaseInElastic.apply(1.0) - 1.0).abs() < 0.001);
    }
}
