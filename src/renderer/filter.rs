//! Event filtering middleware
//!
//! This module provides a middleware mechanism for intercepting and
//! modifying events before they reach the application.

use crossterm::event::Event;

/// Result of filtering an event
#[derive(Debug)]
pub enum FilterResult {
    /// Pass the event through unchanged
    Pass(Event),
    /// Replace the event with a different one
    Replace(Event),
    /// Block the event (don't process it)
    Block,
}

impl FilterResult {
    /// Create a Pass result
    pub fn pass(event: Event) -> Self {
        FilterResult::Pass(event)
    }

    /// Create a Replace result
    pub fn replace(event: Event) -> Self {
        FilterResult::Replace(event)
    }

    /// Create a Block result
    pub fn block() -> Self {
        FilterResult::Block
    }
}

/// Type alias for filter functions
pub type FilterFn = Box<dyn Fn(Event) -> FilterResult + Send + Sync>;

/// An event filter with metadata
pub struct EventFilter {
    /// The filter function
    filter: FilterFn,
    /// Priority (higher = runs first)
    priority: i32,
    /// Name for debugging
    name: String,
}

impl EventFilter {
    /// Create a new event filter
    pub fn new<F>(name: impl Into<String>, filter: F) -> Self
    where
        F: Fn(Event) -> FilterResult + Send + Sync + 'static,
    {
        Self {
            filter: Box::new(filter),
            priority: 0,
            name: name.into(),
        }
    }

    /// Create a new event filter with priority
    pub fn with_priority<F>(name: impl Into<String>, priority: i32, filter: F) -> Self
    where
        F: Fn(Event) -> FilterResult + Send + Sync + 'static,
    {
        Self {
            filter: Box::new(filter),
            priority,
            name: name.into(),
        }
    }

    /// Get the filter name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the filter priority
    pub fn priority(&self) -> i32 {
        self.priority
    }

    /// Apply the filter to an event
    pub fn apply(&self, event: Event) -> FilterResult {
        (self.filter)(event)
    }
}

impl std::fmt::Debug for EventFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventFilter")
            .field("name", &self.name)
            .field("priority", &self.priority)
            .finish()
    }
}

/// A chain of event filters
#[derive(Default)]
pub struct FilterChain {
    filters: Vec<EventFilter>,
}

impl FilterChain {
    /// Create a new empty filter chain
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    /// Add a filter to the chain
    pub fn add(&mut self, filter: EventFilter) {
        self.filters.push(filter);
        // Sort by priority (higher priority first)
        self.filters.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Add a simple filter function
    pub fn add_fn<F>(&mut self, name: impl Into<String>, filter: F)
    where
        F: Fn(Event) -> FilterResult + Send + Sync + 'static,
    {
        self.add(EventFilter::new(name, filter));
    }

    /// Apply all filters to an event
    ///
    /// Returns `Some(event)` if the event should be processed,
    /// or `None` if it was blocked.
    pub fn apply(&self, mut event: Event) -> Option<Event> {
        for filter in &self.filters {
            match filter.apply(event) {
                FilterResult::Pass(e) => event = e,
                FilterResult::Replace(e) => event = e,
                FilterResult::Block => return None,
            }
        }
        Some(event)
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }

    /// Get the number of filters
    pub fn len(&self) -> usize {
        self.filters.len()
    }

    /// Get filter names for debugging
    pub fn filter_names(&self) -> Vec<&str> {
        self.filters.iter().map(|f| f.name()).collect()
    }
}

impl std::fmt::Debug for FilterChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilterChain")
            .field("filters", &self.filter_names())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_key_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn test_filter_result_pass() {
        let event = make_key_event(KeyCode::Char('a'));
        let result = FilterResult::pass(event.clone());
        assert!(matches!(result, FilterResult::Pass(_)));
    }

    #[test]
    fn test_filter_result_replace() {
        let event = make_key_event(KeyCode::Char('a'));
        let result = FilterResult::replace(event.clone());
        assert!(matches!(result, FilterResult::Replace(_)));
    }

    #[test]
    fn test_filter_result_block() {
        let result = FilterResult::block();
        assert!(matches!(result, FilterResult::Block));
    }

    #[test]
    fn test_event_filter_creation() {
        let filter = EventFilter::new("test", FilterResult::Pass);
        assert_eq!(filter.name(), "test");
        assert_eq!(filter.priority(), 0);
    }

    #[test]
    fn test_event_filter_with_priority() {
        let filter = EventFilter::with_priority("test", 10, FilterResult::Pass);
        assert_eq!(filter.name(), "test");
        assert_eq!(filter.priority(), 10);
    }

    #[test]
    fn test_event_filter_apply() {
        let filter = EventFilter::new("blocker", |_| FilterResult::Block);
        let event = make_key_event(KeyCode::Char('a'));
        let result = filter.apply(event);
        assert!(matches!(result, FilterResult::Block));
    }

    #[test]
    fn test_filter_chain_empty() {
        let chain = FilterChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn test_filter_chain_add() {
        let mut chain = FilterChain::new();
        chain.add(EventFilter::new("test", FilterResult::Pass));
        assert!(!chain.is_empty());
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn test_filter_chain_apply_pass() {
        let mut chain = FilterChain::new();
        chain.add_fn("pass", FilterResult::Pass);

        let event = make_key_event(KeyCode::Char('a'));
        let result = chain.apply(event);
        assert!(result.is_some());
    }

    #[test]
    fn test_filter_chain_apply_block() {
        let mut chain = FilterChain::new();
        chain.add_fn("block", |_| FilterResult::Block);

        let event = make_key_event(KeyCode::Char('a'));
        let result = chain.apply(event);
        assert!(result.is_none());
    }

    #[test]
    fn test_filter_chain_apply_replace() {
        let mut chain = FilterChain::new();
        chain.add_fn("replace", |_| {
            FilterResult::Replace(make_key_event(KeyCode::Char('b')))
        });

        let event = make_key_event(KeyCode::Char('a'));
        let result = chain.apply(event);
        assert!(result.is_some());

        if let Some(Event::Key(key)) = result {
            assert_eq!(key.code, KeyCode::Char('b'));
        } else {
            panic!("Expected key event");
        }
    }

    #[test]
    fn test_filter_chain_priority_order() {
        let mut chain = FilterChain::new();

        // Add filters in reverse priority order
        chain.add(EventFilter::with_priority("low", 0, |e| {
            FilterResult::Pass(e)
        }));
        chain.add(EventFilter::with_priority("high", 10, |e| {
            FilterResult::Pass(e)
        }));
        chain.add(EventFilter::with_priority("medium", 5, |e| {
            FilterResult::Pass(e)
        }));

        // Should be sorted by priority (high first)
        let names = chain.filter_names();
        assert_eq!(names, vec!["high", "medium", "low"]);
    }

    #[test]
    fn test_filter_chain_multiple_filters() {
        let mut chain = FilterChain::new();

        // First filter passes
        chain.add_fn("pass1", FilterResult::Pass);

        // Second filter replaces 'a' with 'b'
        chain.add_fn("replace", |e| {
            if let Event::Key(key) = &e {
                if key.code == KeyCode::Char('a') {
                    return FilterResult::Replace(make_key_event(KeyCode::Char('b')));
                }
            }
            FilterResult::Pass(e)
        });

        // Third filter passes
        chain.add_fn("pass2", FilterResult::Pass);

        let event = make_key_event(KeyCode::Char('a'));
        let result = chain.apply(event);

        if let Some(Event::Key(key)) = result {
            assert_eq!(key.code, KeyCode::Char('b'));
        } else {
            panic!("Expected key event");
        }
    }

    #[test]
    fn test_filter_chain_block_stops_chain() {
        let mut chain = FilterChain::new();

        // High priority blocker
        chain.add(EventFilter::with_priority("blocker", 10, |_| {
            FilterResult::Block
        }));

        // Lower priority filter that would replace (should never run)
        chain.add(EventFilter::with_priority("replacer", 0, |_| {
            FilterResult::Replace(make_key_event(KeyCode::Char('x')))
        }));

        let event = make_key_event(KeyCode::Char('a'));
        let result = chain.apply(event);
        assert!(result.is_none());
    }
}
