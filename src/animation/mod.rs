//! Animation system
//!
//! Provides animation primitives for smooth UI transitions.

mod easing;
mod keyframe;
mod spring;

pub use easing::Easing;
pub use keyframe::{
    Animation, AnimationDirection, AnimationInstance, AnimationState, DurationExt, FillMode,
};
pub use spring::{Spring, SpringColor, SpringValue, SpringValue2D};
