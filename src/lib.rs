//! # rnk - React-like Terminal UI for Rust
//!
//! A terminal UI framework inspired by [Ink](https://github.com/vadimdemedes/ink)
//! and [Bubbletea](https://github.com/charmbracelet/bubbletea).
//!
//! ## Public API Boundary
//!
//! Application code should prefer `rnk::prelude::*`. The crate root re-exports
//! a small compatibility surface for common imports, while lower-level modules
//! such as `renderer`, `runtime`, and `testing` are advanced or experimental
//! pre-1.0 APIs. See `docs/API_STABILITY.md` in the repository for the current
//! public API boundary and semver policy.
//!
//! ## API Levels
//!
//! - **Stable app surface**: `rnk::prelude::*`, `rnk::prelude::lite::*`,
//!   `rnk::prelude::widgets::*`, and the root compatibility re-exports listed
//!   below.
//! - **Advanced extension surface**: `core`, `components`, `hooks`, `cmd`,
//!   `animation`, `layout`, and extension macros.
//! - **Experimental/internal-adjacent surface**: `renderer`, `runtime`,
//!   `testing`, and doc-hidden `reconciler` details.
//!
//! New applications should start with the prelude. Use lower-level modules only
//! when building custom components, renderer integrations, or test harnesses.
//!
//! ## Features
//!
//! - Declarative UI with flexbox layout
//! - Reactive state management with hooks
//! - Keyboard and mouse input handling
//! - ANSI color and style support
//! - **Inline mode** (default): Output persists in terminal history
//! - **Fullscreen mode**: Uses alternate screen buffer
//! - **Cross-thread render requests** for async/multi-threaded apps
//! - **Runtime mode switching** between inline and fullscreen
//! - **Println** for persistent messages above the UI
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rnk::prelude::*;
//!
//! fn main() -> std::io::Result<()> {
//!     render(app).run()
//! }
//!
//! fn app() -> Element {
//!     Box::new()
//!         .padding(1)
//!         .child(Text::new("Hello, rnk!").bold().into_element())
//!         .into_element()
//! }
//! ```
//!
//! ## Render Modes
//!
//! ### Inline Mode (Default)
//!
//! Output appears at the current cursor position and persists in terminal history.
//! This is the default mode, matching Ink and Bubbletea's behavior.
//!
//! ```rust,ignore
//! render(app).run()?;           // Inline mode (default)
//! render(app).inline().run()?;  // Explicit inline mode
//! ```
//!
//! ### Fullscreen Mode
//!
//! Uses the alternate screen buffer. Content is cleared when the app exits.
//!
//! ```rust,ignore
//! render(app).fullscreen().run()?;
//! ```
//!
//! ## Runtime Mode Switching
//!
//! Switch between modes at runtime (like Bubbletea):
//!
//! ```rust,ignore
//! let app = use_app();
//!
//! use_input(move |input, _key| {
//!     if input == " " {
//!         if app.is_alt_screen() {
//!             app.exit_alt_screen();  // Switch to inline
//!         } else {
//!             app.enter_alt_screen(); // Switch to fullscreen
//!         }
//!     }
//! });
//! ```
//!
//! ## Println for Persistent Messages
//!
//! In inline mode, use `println()` to output messages above the UI:
//!
//! ```rust,ignore
//! use rnk::println;
//!
//! // In an input handler
//! rnk::println("Task completed!");
//! rnk::println(format!("Downloaded {} files", count));
//!
//! // Or via AppContext
//! let app = use_app();
//! app.println("Another message");
//! ```
//!
//! ## Cross-Thread Rendering
//!
//! When updating state from a background thread, use `request_render()` to
//! notify the UI to refresh:
//!
//! ```rust,ignore
//! use std::thread;
//! use std::sync::{Arc, RwLock};
//! use rnk::request_render;
//!
//! let state = Arc::new(RwLock::new(0));
//! let state_clone = Arc::clone(&state);
//!
//! thread::spawn(move || {
//!     *state_clone.write().unwrap() += 1;
//!     request_render(); // Notify rnk to re-render
//! });
//! ```

/// Advanced extension surface for keyframe, easing, and spring animation APIs.
pub mod animation;
/// Advanced extension surface for typed side-effect commands.
pub mod cmd;
/// Advanced extension surface for component modules and concrete widgets.
///
/// Application examples should prefer `rnk::prelude::*` or
/// `rnk::prelude::widgets::*` unless they are documenting a specific component.
pub mod components;
/// Advanced extension surface for core element, style, color, and layout types.
pub mod core;
/// Advanced extension surface for hook implementations and hook helper types.
pub mod hooks;
/// Advanced extension surface for measurement and Taffy-backed layout details.
pub mod layout;
/// Stable convenience macros for declarative UI construction.
#[macro_use]
pub mod macros;
#[doc(hidden)]
pub mod reconciler;
/// Experimental advanced surface for renderer controls and terminal buffers.
///
/// Prefer root render entry points and `rnk::prelude::*` for normal
/// applications. Direct renderer types may change before `1.0`.
pub mod renderer;
/// Experimental internal-adjacent surface for runtime state and terminal
/// environment helpers.
pub mod runtime;

/// Experimental test-support surface for snapshots, harnesses, and render
/// assertions.
pub mod testing;

/// Stable application import surfaces.
pub mod prelude;

// Root compatibility re-exports for common application imports.
pub use crate::components::{Box, Text};
pub use crate::core::{AccessibilityProps, AccessibilityRole, Color, Element, ElementId, Style};

// Root compatibility re-exports for rendering and app-control APIs.
pub use crate::renderer::{
    AppBuilder,
    AppOptions,
    IntoPrintable,
    ModeSwitch,
    Printable,
    // Types
    RenderHandle,
    RenderOptions,
    enter_alt_screen,
    exit_alt_screen,
    is_alt_screen,
    println,
    println_trimmed,
    // Main entry points
    render,
    render_fullscreen,
    render_handle,
    render_inline,
    // Element rendering APIs
    render_to_string,
    render_to_string_auto,
    render_to_string_no_trim,
    render_to_string_raw,
    render_to_string_with_options,
    // Cross-thread APIs
    request_render,
};
