//! Frame rate control and statistics
//!
//! This module provides configurable frame rate control with statistics
//! collection and adaptive frame rate support.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Configuration for frame rate control
#[derive(Debug, Clone)]
pub struct FrameRateConfig {
    /// Target frames per second (1-120, default: 60)
    pub target_fps: u32,
    /// Enable adaptive frame rate (adjusts based on render time)
    pub adaptive: bool,
    /// Minimum FPS when adaptive is enabled (default: 10)
    pub min_fps: u32,
    /// Maximum FPS when adaptive is enabled (default: 120)
    pub max_fps: u32,
    /// Collect frame statistics (default: false)
    pub collect_stats: bool,
}

impl Default for FrameRateConfig {
    fn default() -> Self {
        Self {
            target_fps: 60,
            adaptive: false,
            min_fps: 10,
            max_fps: 120,
            collect_stats: false,
        }
    }
}

impl FrameRateConfig {
    /// Create a new config with the specified target FPS
    pub fn new(target_fps: u32) -> Self {
        Self {
            target_fps: target_fps.clamp(1, 120),
            ..Default::default()
        }
    }

    /// Enable adaptive frame rate
    pub fn adaptive(mut self, min_fps: u32, max_fps: u32) -> Self {
        self.adaptive = true;
        self.min_fps = min_fps.clamp(1, 120);
        self.max_fps = max_fps.clamp(self.min_fps, 120);
        self
    }

    /// Enable statistics collection
    pub fn with_stats(mut self) -> Self {
        self.collect_stats = true;
        self
    }
}

/// Frame rate statistics
#[derive(Debug, Clone, Default)]
pub struct FrameRateStats {
    /// Current measured FPS
    pub current_fps: f64,
    /// Average frame time in milliseconds
    pub avg_frame_time_ms: f64,
    /// Number of dropped frames (frames that took longer than target)
    pub dropped_frames: u64,
    /// Total frames rendered
    pub total_frames: u64,
    /// Minimum frame time in milliseconds (best case)
    pub min_frame_time_ms: f64,
    /// Maximum frame time in milliseconds (worst case)
    pub max_frame_time_ms: f64,
}

/// Shared frame rate statistics for cross-thread access
#[derive(Debug, Default)]
pub struct SharedFrameRateStats {
    current_fps: AtomicU64,
    avg_frame_time_ms: AtomicU64,
    dropped_frames: AtomicU64,
    total_frames: AtomicU64,
    min_frame_time_ms: AtomicU64,
    max_frame_time_ms: AtomicU64,
}

impl SharedFrameRateStats {
    /// Create new shared stats
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            current_fps: AtomicU64::new(0),
            avg_frame_time_ms: AtomicU64::new(0),
            dropped_frames: AtomicU64::new(0),
            total_frames: AtomicU64::new(0),
            min_frame_time_ms: AtomicU64::new(f64::MAX.to_bits()),
            max_frame_time_ms: AtomicU64::new(0),
        })
    }

    /// Get a snapshot of the current stats
    pub fn snapshot(&self) -> FrameRateStats {
        FrameRateStats {
            current_fps: f64::from_bits(self.current_fps.load(Ordering::Relaxed)),
            avg_frame_time_ms: f64::from_bits(self.avg_frame_time_ms.load(Ordering::Relaxed)),
            dropped_frames: self.dropped_frames.load(Ordering::Relaxed),
            total_frames: self.total_frames.load(Ordering::Relaxed),
            min_frame_time_ms: f64::from_bits(self.min_frame_time_ms.load(Ordering::Relaxed)),
            max_frame_time_ms: f64::from_bits(self.max_frame_time_ms.load(Ordering::Relaxed)),
        }
    }

    /// Update stats from local stats
    fn update(&self, stats: &FrameRateStats) {
        self.current_fps
            .store(stats.current_fps.to_bits(), Ordering::Relaxed);
        self.avg_frame_time_ms
            .store(stats.avg_frame_time_ms.to_bits(), Ordering::Relaxed);
        self.dropped_frames
            .store(stats.dropped_frames, Ordering::Relaxed);
        self.total_frames
            .store(stats.total_frames, Ordering::Relaxed);
        self.min_frame_time_ms
            .store(stats.min_frame_time_ms.to_bits(), Ordering::Relaxed);
        self.max_frame_time_ms
            .store(stats.max_frame_time_ms.to_bits(), Ordering::Relaxed);
    }
}

/// Frame rate controller
///
/// Manages frame timing, adaptive frame rate, and statistics collection.
pub struct FrameRateController {
    config: FrameRateConfig,
    last_frame: Instant,
    frame_times: VecDeque<Duration>,
    current_target_fps: u32,
    stats: FrameRateStats,
    shared_stats: Option<Arc<SharedFrameRateStats>>,
}

impl FrameRateController {
    /// Create a new frame rate controller
    pub fn new(config: FrameRateConfig) -> Self {
        let current_target_fps = config.target_fps;
        let shared_stats = if config.collect_stats {
            Some(SharedFrameRateStats::new())
        } else {
            None
        };

        Self {
            config,
            last_frame: Instant::now(),
            frame_times: VecDeque::with_capacity(60),
            current_target_fps,
            stats: FrameRateStats::default(),
            shared_stats,
        }
    }

    /// Get the shared stats handle (for use_frame_rate hook)
    pub fn shared_stats(&self) -> Option<Arc<SharedFrameRateStats>> {
        self.shared_stats.clone()
    }

    /// Get the current target frame duration
    pub fn frame_duration(&self) -> Duration {
        Duration::from_millis(1000 / self.current_target_fps as u64)
    }

    /// Get the current target FPS
    pub fn current_fps(&self) -> u32 {
        self.current_target_fps
    }

    /// Check if enough time has passed for the next frame
    pub fn should_render(&self) -> bool {
        self.last_frame.elapsed() >= self.frame_duration()
    }

    /// Record a frame render and update statistics
    ///
    /// Call this after each frame render with the time taken to render.
    pub fn record_frame(&mut self, render_time: Duration) {
        let now = Instant::now();
        let frame_time = now.duration_since(self.last_frame);
        self.last_frame = now;

        // Update frame times buffer (keep last 60 frames)
        self.frame_times.push_back(frame_time);
        if self.frame_times.len() > 60 {
            self.frame_times.pop_front();
        }

        // Update statistics
        self.stats.total_frames += 1;

        let frame_time_ms = frame_time.as_secs_f64() * 1000.0;
        let target_frame_time_ms = 1000.0 / self.current_target_fps as f64;

        // Check for dropped frame
        if frame_time_ms > target_frame_time_ms * 1.5 {
            self.stats.dropped_frames += 1;
        }

        // Update min/max
        if frame_time_ms < self.stats.min_frame_time_ms || self.stats.total_frames == 1 {
            self.stats.min_frame_time_ms = frame_time_ms;
        }
        if frame_time_ms > self.stats.max_frame_time_ms {
            self.stats.max_frame_time_ms = frame_time_ms;
        }

        // Calculate average frame time and current FPS
        if !self.frame_times.is_empty() {
            let total: Duration = self.frame_times.iter().sum();
            let avg = total / self.frame_times.len() as u32;
            self.stats.avg_frame_time_ms = avg.as_secs_f64() * 1000.0;
            self.stats.current_fps = 1000.0 / self.stats.avg_frame_time_ms;
        }

        // Adaptive frame rate adjustment
        if self.config.adaptive {
            self.adjust_frame_rate(render_time);
        }

        // Update shared stats
        if let Some(ref shared) = self.shared_stats {
            shared.update(&self.stats);
        }
    }

    /// Adjust frame rate based on render performance
    fn adjust_frame_rate(&mut self, render_time: Duration) {
        let render_time_ms = render_time.as_secs_f64() * 1000.0;
        let target_frame_time_ms = 1000.0 / self.current_target_fps as f64;

        // If render takes more than 80% of frame budget, decrease FPS
        if render_time_ms > target_frame_time_ms * 0.8 {
            let new_fps = (self.current_target_fps as f64 * 0.9) as u32;
            self.current_target_fps = new_fps.clamp(self.config.min_fps, self.config.max_fps);
        }
        // If render takes less than 50% of frame budget, try increasing FPS
        else if render_time_ms < target_frame_time_ms * 0.5
            && self.current_target_fps < self.config.target_fps
        {
            let new_fps = (self.current_target_fps as f64 * 1.1) as u32;
            self.current_target_fps = new_fps.clamp(self.config.min_fps, self.config.max_fps);
        }
    }

    /// Get current statistics
    pub fn stats(&self) -> &FrameRateStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = FrameRateStats::default();
        self.frame_times.clear();
        if let Some(ref shared) = self.shared_stats {
            shared.update(&self.stats);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_rate_config_default() {
        let config = FrameRateConfig::default();
        assert_eq!(config.target_fps, 60);
        assert!(!config.adaptive);
        assert!(!config.collect_stats);
    }

    #[test]
    fn test_frame_rate_config_new() {
        let config = FrameRateConfig::new(30);
        assert_eq!(config.target_fps, 30);
    }

    #[test]
    fn test_frame_rate_config_clamp() {
        let config = FrameRateConfig::new(200);
        assert_eq!(config.target_fps, 120);

        let config = FrameRateConfig::new(0);
        assert_eq!(config.target_fps, 1);
    }

    #[test]
    fn test_frame_rate_config_adaptive() {
        let config = FrameRateConfig::new(60).adaptive(15, 90);
        assert!(config.adaptive);
        assert_eq!(config.min_fps, 15);
        assert_eq!(config.max_fps, 90);
    }

    #[test]
    fn test_frame_rate_config_with_stats() {
        let config = FrameRateConfig::new(60).with_stats();
        assert!(config.collect_stats);
    }

    #[test]
    fn test_frame_rate_controller_creation() {
        let controller = FrameRateController::new(FrameRateConfig::default());
        assert_eq!(controller.current_fps(), 60);
        assert!(controller.shared_stats().is_none());
    }

    #[test]
    fn test_frame_rate_controller_with_stats() {
        let config = FrameRateConfig::new(60).with_stats();
        let controller = FrameRateController::new(config);
        assert!(controller.shared_stats().is_some());
    }

    #[test]
    fn test_frame_duration() {
        let controller = FrameRateController::new(FrameRateConfig::new(60));
        assert_eq!(controller.frame_duration(), Duration::from_millis(16));

        let controller = FrameRateController::new(FrameRateConfig::new(30));
        assert_eq!(controller.frame_duration(), Duration::from_millis(33));
    }

    #[test]
    fn test_record_frame() {
        let config = FrameRateConfig::new(60).with_stats();
        let mut controller = FrameRateController::new(config);

        // Simulate a few frames
        for _ in 0..5 {
            std::thread::sleep(Duration::from_millis(10));
            controller.record_frame(Duration::from_millis(5));
        }

        assert_eq!(controller.stats().total_frames, 5);
        assert!(controller.stats().current_fps > 0.0);
    }

    #[test]
    fn test_shared_stats_snapshot() {
        let config = FrameRateConfig::new(60).with_stats();
        let mut controller = FrameRateController::new(config);
        let shared = controller.shared_stats().unwrap();

        controller.record_frame(Duration::from_millis(5));

        let snapshot = shared.snapshot();
        assert_eq!(snapshot.total_frames, 1);
    }

    #[test]
    fn test_reset_stats() {
        let config = FrameRateConfig::new(60).with_stats();
        let mut controller = FrameRateController::new(config);

        controller.record_frame(Duration::from_millis(5));
        assert_eq!(controller.stats().total_frames, 1);

        controller.reset_stats();
        assert_eq!(controller.stats().total_frames, 0);
    }
}
