//! Timer and Stopwatch components
//!
//! Provides countdown timer and stopwatch functionality for TUI applications.

use std::time::{Duration, Instant};

/// Timer state for countdown functionality
#[derive(Debug, Clone)]
pub struct TimerState {
    /// Total duration of the timer
    pub duration: Duration,
    /// Remaining time
    pub remaining: Duration,
    /// Whether the timer is running
    pub running: bool,
    /// When the timer was last started/resumed
    start_instant: Option<Instant>,
    /// Time remaining when paused
    paused_remaining: Duration,
}

impl TimerState {
    /// Create a new timer with the specified duration
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            remaining: duration,
            running: false,
            start_instant: None,
            paused_remaining: duration,
        }
    }

    /// Create a timer from seconds
    pub fn from_secs(secs: u64) -> Self {
        Self::new(Duration::from_secs(secs))
    }

    /// Create a timer from minutes
    pub fn from_mins(mins: u64) -> Self {
        Self::new(Duration::from_secs(mins * 60))
    }

    /// Start or resume the timer
    pub fn start(&mut self) {
        if !self.running && self.paused_remaining > Duration::ZERO {
            self.running = true;
            self.start_instant = Some(Instant::now());
        }
    }

    /// Pause the timer
    pub fn pause(&mut self) {
        if self.running {
            self.running = false;
            self.paused_remaining = self.remaining;
            self.start_instant = None;
        }
    }

    /// Toggle between running and paused
    pub fn toggle(&mut self) {
        if self.running {
            self.pause();
        } else {
            self.start();
        }
    }

    /// Reset the timer to its initial duration
    pub fn reset(&mut self) {
        self.running = false;
        self.remaining = self.duration;
        self.paused_remaining = self.duration;
        self.start_instant = None;
    }

    /// Update the timer state (call this each frame)
    pub fn tick(&mut self) {
        if self.running {
            if let Some(start) = self.start_instant {
                let elapsed = start.elapsed();
                self.remaining = self.paused_remaining.saturating_sub(elapsed);

                if self.remaining == Duration::ZERO {
                    self.running = false;
                    self.start_instant = None;
                }
            }
        }
    }

    /// Check if the timer has finished
    pub fn is_finished(&self) -> bool {
        self.remaining == Duration::ZERO
    }

    /// Get the progress as a fraction (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        if self.duration.is_zero() {
            return 1.0;
        }
        1.0 - (self.remaining.as_secs_f64() / self.duration.as_secs_f64())
    }

    /// Get remaining time formatted as MM:SS
    pub fn format_mmss(&self) -> String {
        let total_secs = self.remaining.as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    /// Get remaining time formatted as HH:MM:SS
    pub fn format_hhmmss(&self) -> String {
        let total_secs = self.remaining.as_secs();
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        let secs = total_secs % 60;
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    }

    /// Get remaining time formatted with milliseconds
    pub fn format_precise(&self) -> String {
        let total_millis = self.remaining.as_millis();
        let mins = total_millis / 60000;
        let secs = (total_millis % 60000) / 1000;
        let millis = total_millis % 1000;
        format!("{:02}:{:02}.{:03}", mins, secs, millis)
    }
}

/// Stopwatch state for elapsed time tracking
#[derive(Debug, Clone)]
pub struct StopwatchState {
    /// Total elapsed time
    pub elapsed: Duration,
    /// Whether the stopwatch is running
    pub running: bool,
    /// When the stopwatch was last started/resumed
    start_instant: Option<Instant>,
    /// Accumulated time from previous runs
    accumulated: Duration,
    /// Lap times
    laps: Vec<Duration>,
}

impl Default for StopwatchState {
    fn default() -> Self {
        Self::new()
    }
}

impl StopwatchState {
    /// Create a new stopwatch
    pub fn new() -> Self {
        Self {
            elapsed: Duration::ZERO,
            running: false,
            start_instant: None,
            accumulated: Duration::ZERO,
            laps: Vec::new(),
        }
    }

    /// Start or resume the stopwatch
    pub fn start(&mut self) {
        if !self.running {
            self.running = true;
            self.start_instant = Some(Instant::now());
        }
    }

    /// Pause the stopwatch
    pub fn pause(&mut self) {
        if self.running {
            self.running = false;
            self.accumulated = self.elapsed;
            self.start_instant = None;
        }
    }

    /// Toggle between running and paused
    pub fn toggle(&mut self) {
        if self.running {
            self.pause();
        } else {
            self.start();
        }
    }

    /// Reset the stopwatch
    pub fn reset(&mut self) {
        self.running = false;
        self.elapsed = Duration::ZERO;
        self.accumulated = Duration::ZERO;
        self.start_instant = None;
        self.laps.clear();
    }

    /// Record a lap time
    pub fn lap(&mut self) {
        if self.running || self.elapsed > Duration::ZERO {
            self.laps.push(self.elapsed);
        }
    }

    /// Get all lap times
    pub fn laps(&self) -> &[Duration] {
        &self.laps
    }

    /// Get the last lap time
    pub fn last_lap(&self) -> Option<Duration> {
        self.laps.last().copied()
    }

    /// Get lap split times (time between laps)
    pub fn splits(&self) -> Vec<Duration> {
        let mut splits = Vec::new();
        let mut prev = Duration::ZERO;
        for &lap in &self.laps {
            splits.push(lap.saturating_sub(prev));
            prev = lap;
        }
        splits
    }

    /// Update the stopwatch state (call this each frame)
    pub fn tick(&mut self) {
        if self.running {
            if let Some(start) = self.start_instant {
                self.elapsed = self.accumulated + start.elapsed();
            }
        }
    }

    /// Get elapsed time formatted as MM:SS
    pub fn format_mmss(&self) -> String {
        let total_secs = self.elapsed.as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    /// Get elapsed time formatted as HH:MM:SS
    pub fn format_hhmmss(&self) -> String {
        let total_secs = self.elapsed.as_secs();
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        let secs = total_secs % 60;
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    }

    /// Get elapsed time formatted with milliseconds
    pub fn format_precise(&self) -> String {
        let total_millis = self.elapsed.as_millis();
        let mins = total_millis / 60000;
        let secs = (total_millis % 60000) / 1000;
        let millis = total_millis % 1000;
        format!("{:02}:{:02}.{:03}", mins, secs, millis)
    }

    /// Get elapsed time formatted with centiseconds (like a real stopwatch)
    pub fn format_stopwatch(&self) -> String {
        let total_centis = self.elapsed.as_millis() / 10;
        let mins = total_centis / 6000;
        let secs = (total_centis % 6000) / 100;
        let centis = total_centis % 100;
        format!("{:02}:{:02}.{:02}", mins, secs, centis)
    }
}

/// Format a duration as MM:SS
pub fn format_duration_mmss(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", mins, secs)
}

/// Format a duration as HH:MM:SS
pub fn format_duration_hhmmss(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

/// Format a duration with milliseconds
pub fn format_duration_precise(duration: Duration) -> String {
    let total_millis = duration.as_millis();
    let mins = total_millis / 60000;
    let secs = (total_millis % 60000) / 1000;
    let millis = total_millis % 1000;
    format!("{:02}:{:02}.{:03}", mins, secs, millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_creation() {
        let timer = TimerState::new(Duration::from_secs(60));
        assert_eq!(timer.duration, Duration::from_secs(60));
        assert_eq!(timer.remaining, Duration::from_secs(60));
        assert!(!timer.running);
    }

    #[test]
    fn test_timer_from_secs() {
        let timer = TimerState::from_secs(30);
        assert_eq!(timer.duration, Duration::from_secs(30));
    }

    #[test]
    fn test_timer_from_mins() {
        let timer = TimerState::from_mins(5);
        assert_eq!(timer.duration, Duration::from_secs(300));
    }

    #[test]
    fn test_timer_start_pause() {
        let mut timer = TimerState::from_secs(60);
        assert!(!timer.running);

        timer.start();
        assert!(timer.running);

        timer.pause();
        assert!(!timer.running);
    }

    #[test]
    fn test_timer_toggle() {
        let mut timer = TimerState::from_secs(60);
        assert!(!timer.running);

        timer.toggle();
        assert!(timer.running);

        timer.toggle();
        assert!(!timer.running);
    }

    #[test]
    fn test_timer_reset() {
        let mut timer = TimerState::from_secs(60);
        timer.start();
        timer.remaining = Duration::from_secs(30);
        timer.reset();

        assert!(!timer.running);
        assert_eq!(timer.remaining, Duration::from_secs(60));
    }

    #[test]
    fn test_timer_progress() {
        let mut timer = TimerState::from_secs(100);
        assert_eq!(timer.progress(), 0.0);

        timer.remaining = Duration::from_secs(50);
        assert!((timer.progress() - 0.5).abs() < 0.001);

        timer.remaining = Duration::ZERO;
        assert_eq!(timer.progress(), 1.0);
    }

    #[test]
    fn test_timer_is_finished() {
        let mut timer = TimerState::from_secs(60);
        assert!(!timer.is_finished());

        timer.remaining = Duration::ZERO;
        assert!(timer.is_finished());
    }

    #[test]
    fn test_timer_format_mmss() {
        let mut timer = TimerState::from_secs(125);
        assert_eq!(timer.format_mmss(), "02:05");

        timer.remaining = Duration::from_secs(3661);
        assert_eq!(timer.format_mmss(), "61:01");
    }

    #[test]
    fn test_timer_format_hhmmss() {
        let timer = TimerState::new(Duration::from_secs(3661));
        assert_eq!(timer.format_hhmmss(), "01:01:01");
    }

    #[test]
    fn test_stopwatch_creation() {
        let sw = StopwatchState::new();
        assert_eq!(sw.elapsed, Duration::ZERO);
        assert!(!sw.running);
    }

    #[test]
    fn test_stopwatch_start_pause() {
        let mut sw = StopwatchState::new();
        assert!(!sw.running);

        sw.start();
        assert!(sw.running);

        sw.pause();
        assert!(!sw.running);
    }

    #[test]
    fn test_stopwatch_toggle() {
        let mut sw = StopwatchState::new();
        assert!(!sw.running);

        sw.toggle();
        assert!(sw.running);

        sw.toggle();
        assert!(!sw.running);
    }

    #[test]
    fn test_stopwatch_reset() {
        let mut sw = StopwatchState::new();
        sw.start();
        sw.elapsed = Duration::from_secs(30);
        sw.lap();
        sw.reset();

        assert!(!sw.running);
        assert_eq!(sw.elapsed, Duration::ZERO);
        assert!(sw.laps().is_empty());
    }

    #[test]
    fn test_stopwatch_laps() {
        let mut sw = StopwatchState::new();
        sw.elapsed = Duration::from_secs(10);
        sw.lap();
        sw.elapsed = Duration::from_secs(25);
        sw.lap();
        sw.elapsed = Duration::from_secs(45);
        sw.lap();

        assert_eq!(sw.laps().len(), 3);
        assert_eq!(sw.last_lap(), Some(Duration::from_secs(45)));
    }

    #[test]
    fn test_stopwatch_splits() {
        let mut sw = StopwatchState::new();
        sw.elapsed = Duration::from_secs(10);
        sw.lap();
        sw.elapsed = Duration::from_secs(25);
        sw.lap();
        sw.elapsed = Duration::from_secs(45);
        sw.lap();

        let splits = sw.splits();
        assert_eq!(splits.len(), 3);
        assert_eq!(splits[0], Duration::from_secs(10));
        assert_eq!(splits[1], Duration::from_secs(15));
        assert_eq!(splits[2], Duration::from_secs(20));
    }

    #[test]
    fn test_stopwatch_format() {
        let mut sw = StopwatchState::new();
        sw.elapsed = Duration::from_millis(125456);

        assert_eq!(sw.format_mmss(), "02:05");
        assert_eq!(sw.format_hhmmss(), "00:02:05");
        assert_eq!(sw.format_precise(), "02:05.456");
        assert_eq!(sw.format_stopwatch(), "02:05.45");
    }

    #[test]
    fn test_format_duration_helpers() {
        let d = Duration::from_secs(3661);
        assert_eq!(format_duration_mmss(d), "61:01");
        assert_eq!(format_duration_hhmmss(d), "01:01:01");

        let d = Duration::from_millis(125456);
        assert_eq!(format_duration_precise(d), "02:05.456");
    }
}
