//! Notification/Toast demo - Temporary messages
//!
//! Run: cargo run --example notification_demo

use rnk::components::{
    Notification, NotificationBorder, NotificationItem, NotificationLevel, NotificationPosition,
    NotificationState, NotificationStyle, Toast,
};

fn main() {
    println!("=== Notification/Toast Component Demo ===\n");

    // Notification levels
    println!("--- Notification Levels ---");
    let levels = [
        NotificationLevel::Info,
        NotificationLevel::Success,
        NotificationLevel::Warning,
        NotificationLevel::Error,
    ];

    for level in levels {
        println!("  {:?}:", level);
        println!("    icon: {}", level.icon());
        println!("    label: {}", level.label());
        println!();
    }

    // Simple toasts
    println!("--- Simple Toasts ---\n");

    let items = [
        NotificationItem::info("1", "This is an info message"),
        NotificationItem::success("2", "Operation completed successfully"),
        NotificationItem::warning("3", "Please review your changes"),
        NotificationItem::error("4", "Failed to save file"),
    ];

    for item in &items {
        let toast = Toast::new(item);
        println!("{}", toast.render());
    }

    // Toast with title
    println!("--- Toast with Title ---\n");
    let item = NotificationItem::success("5", "Your changes have been saved").title("Auto-save");
    let toast = Toast::new(&item);
    println!("{}\n", toast.render());

    // Style presets
    println!("--- Style Presets ---\n");

    let item = NotificationItem::info("6", "Hello, World!");

    println!("NotificationStyle::default():");
    let toast = Toast::new(&item).style(NotificationStyle::default());
    println!("{}", toast.render());

    println!("NotificationStyle::minimal():");
    let toast = Toast::new(&item).style(NotificationStyle::minimal());
    println!("{}\n", toast.render());

    println!("NotificationStyle::compact():");
    let toast = Toast::new(&item).style(NotificationStyle::compact());
    println!("{}", toast.render());

    println!("NotificationStyle::detailed():");
    let toast = Toast::new(&item).style(NotificationStyle::detailed());
    println!("{}", toast.render());

    // Border styles
    println!("--- Border Styles ---\n");

    let borders = [
        ("None", NotificationBorder::None),
        ("Single", NotificationBorder::Single),
        ("Double", NotificationBorder::Double),
        ("Rounded", NotificationBorder::Rounded),
        ("Heavy", NotificationBorder::Heavy),
    ];

    let item = NotificationItem::success("7", "Border demo");

    for (name, border) in borders {
        println!("{}:", name);
        let style = NotificationStyle::new().border(border);
        let toast = Toast::new(&item).style(style);
        println!("{}", toast.render());
    }

    // NotificationState
    println!("--- NotificationState API ---\n");

    let mut state = NotificationState::new();
    println!("NotificationState::new():");
    println!("  is_empty: {}", state.is_empty());
    println!("  count: {}", state.count());
    println!();

    // Add notifications
    let id1 = state.info("First notification", 1000);
    let id2 = state.success("Second notification", 1500);
    let _id3 = state.warning("Third notification", 2000);

    println!("After adding 3 notifications:");
    println!("  count: {}", state.count());
    println!("  visible: {}", state.visible().len());
    println!();

    // Dismiss
    println!("dismiss(\"{}\"):", id1);
    state.dismiss(&id1);
    println!("  count: {}", state.count());
    println!();

    // Auto-dismiss simulation
    println!("--- Auto-dismiss Simulation ---");
    println!("  (notifications have 3000ms default duration)\n");

    let mut state = NotificationState::new();
    state.info("Message at t=0", 0);
    state.success("Message at t=1000", 1000);

    println!("  t=0ms: count = {}", state.count());

    state.update(2000);
    println!("  t=2000ms: count = {}", state.count());

    state.update(3500);
    println!("  t=3500ms: count = {} (first expired)", state.count());

    state.update(4500);
    println!("  t=4500ms: count = {} (second expired)", state.count());
    println!();

    // Max visible
    println!("--- Max Visible Limit ---");
    let mut state = NotificationState::new().max_visible(2);

    state.info("Notification 1", 0);
    state.info("Notification 2", 0);
    state.info("Notification 3", 0);
    state.info("Notification 4", 0);

    println!("  Added 4 notifications with max_visible=2:");
    println!("  total count: {}", state.count());
    println!("  visible count: {}", state.visible().len());
    println!();

    // Positions
    println!("--- Notification Positions ---");
    let positions = [
        NotificationPosition::Top,
        NotificationPosition::TopRight,
        NotificationPosition::TopLeft,
        NotificationPosition::Bottom,
        NotificationPosition::BottomRight,
        NotificationPosition::BottomLeft,
    ];

    for pos in positions {
        println!("  {:?}", pos);
    }
    println!();

    // Notification container
    println!("--- Notification Container ---\n");

    let mut state = NotificationState::new();
    state.info("Info message", 0);
    state.success("Success message", 0);
    state.error("Error message", 0);

    let notification = Notification::new(&state);
    println!("{}", notification.render());

    // Visual representation
    println!("--- Visual Representation ---");
    println!("  (what it would look like in a TUI app)\n");

    // Simulated screen with notification
    println!("  ┌────────────────────────────────────────┐");
    println!("  │  My Application                        │");
    println!("  │                                        │");
    println!("  │  Content here...                       │");
    println!("  │                                        │");
    println!("  │                    \x1b[32m╭──────────────────╮\x1b[0m");
    println!("  │                    \x1b[32m│ ✓ File saved!    │\x1b[0m");
    println!("  │                    \x1b[32m╰──────────────────╯\x1b[0m");
    println!("  └────────────────────────────────────────┘");
    println!();

    // Usage example
    println!("--- Usage in TUI App ---");
    println!("```rust");
    println!("use rnk::components::{{Notification, NotificationState, NotificationItem}};");
    println!("use rnk::hooks::{{use_signal, use_interval}};");
    println!();
    println!("fn app() -> Element {{");
    println!("    let notifications = use_signal(|| NotificationState::new());");
    println!("    let time = use_signal(|| 0u64);");
    println!();
    println!("    // Update time and auto-dismiss");
    println!("    use_interval(100, move || {{");
    println!("        let t = time.get() + 100;");
    println!("        time.set(t);");
    println!("        let mut n = notifications.get();");
    println!("        n.update(t);");
    println!("        notifications.set(n);");
    println!("    }});");
    println!();
    println!("    // Show notification on action");
    println!("    let show_toast = move || {{");
    println!("        let mut n = notifications.get();");
    println!("        n.success(\"Action completed!\", time.get());");
    println!("        notifications.set(n);");
    println!("    }};");
    println!();
    println!("    let n = notifications.get();");
    println!("    col![");
    println!("        Text::new(\"My App\"),");
    println!("        Notification::new(&n),");
    println!("    ].into_element()");
    println!("}}");
    println!("```");
}
