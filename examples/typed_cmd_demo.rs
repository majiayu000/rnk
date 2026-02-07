//! TypedCmd Demo - Type-safe message passing
//!
//! This example demonstrates the TypedCmd system for type-safe
//! async operations that return typed messages.

use rnk::cmd::{AppMsg, TypedCmd};
/// Application messages
#[derive(Debug, Clone)]
enum Msg {
    /// Data was loaded successfully
    DataLoaded(Vec<String>),
    /// Loading failed with error
    LoadError(String),
    /// Timer tick
    Tick(u64),
    /// User requested refresh
    Refresh,
}

/// Simulated async data loading
fn load_data() -> TypedCmd<Msg> {
    TypedCmd::perform(|| async {
        // Simulate async operation
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Msg::DataLoaded(vec![
            "Item 1".to_string(),
            "Item 2".to_string(),
            "Item 3".to_string(),
        ])
    })
}

/// Create a timer that produces Tick messages
fn start_timer(count: u64) -> TypedCmd<Msg> {
    TypedCmd::tick(std::time::Duration::from_secs(1), move |_| {
        Msg::Tick(count + 1)
    })
}

/// Batch multiple commands
fn init_commands() -> TypedCmd<Msg> {
    TypedCmd::batch(vec![load_data(), start_timer(0)])
}

/// Map child messages to parent messages
#[derive(Debug)]
enum ParentMsg {
    Child(Msg),
    Other,
}

fn describe_msg(msg: &Msg) -> String {
    match msg {
        Msg::DataLoaded(items) => format!("Loaded {} items", items.len()),
        Msg::LoadError(err) => format!("Error: {}", err),
        Msg::Tick(tick) => format!("Tick {}", tick),
        Msg::Refresh => "Refresh requested".to_string(),
    }
}

fn describe_parent_msg(msg: &ParentMsg) -> String {
    match msg {
        ParentMsg::Child(child) => format!("Child({})", describe_msg(child)),
        ParentMsg::Other => "Other".to_string(),
    }
}

fn map_example() -> TypedCmd<ParentMsg> {
    // Child command produces Msg
    let child_cmd: TypedCmd<Msg> = load_data();

    // Map to parent message type
    child_cmd.map(ParentMsg::Child)
}

fn main() {
    println!("TypedCmd Demo - Type-safe message passing\n");

    // Demonstrate creating various typed commands
    println!("1. Creating a perform command:");
    let cmd: TypedCmd<Msg> = load_data();
    println!("   {:?}\n", cmd);

    println!("2. Creating a tick command:");
    let cmd: TypedCmd<Msg> = start_timer(0);
    println!("   {:?}\n", cmd);

    println!("3. Batching commands:");
    let cmd: TypedCmd<Msg> = init_commands();
    println!("   {:?}\n", cmd);

    println!("4. Mapping message types:");
    let cmd: TypedCmd<ParentMsg> = map_example();
    println!("   {:?}\n", cmd);

    println!("5. Sequencing commands:");
    let cmd: TypedCmd<Msg> = TypedCmd::sequence(vec![
        TypedCmd::sleep(std::time::Duration::from_millis(100)),
        load_data(),
    ]);
    println!("   {:?}\n", cmd);

    println!("6. Chaining with and_then:");
    let cmd: TypedCmd<Msg> =
        TypedCmd::sleep(std::time::Duration::from_millis(50)).and_then(load_data());
    println!("   {:?}\n", cmd);

    println!("7. Sample message values:");
    let sample_msgs = [
        Msg::DataLoaded(vec!["Preview".to_string()]),
        Msg::LoadError("network timeout".to_string()),
        Msg::Tick(42),
        Msg::Refresh,
    ];
    for msg in &sample_msgs {
        println!("   {}", describe_msg(msg));
    }

    let sample_parent_msgs = [ParentMsg::Child(Msg::Refresh), ParentMsg::Other];
    for msg in &sample_parent_msgs {
        println!("   {}", describe_parent_msg(msg));
    }
    println!();

    // Show AppMsg usage
    println!("8. Built-in AppMsg types:");
    let msgs = vec![
        AppMsg::WindowResize {
            width: 80,
            height: 24,
        },
        AppMsg::KeyInput("q".to_string()),
        AppMsg::Tick(std::time::Instant::now()),
        AppMsg::FocusChanged(Some("input-1".to_string())),
        AppMsg::Blur,
        AppMsg::None,
    ];
    for msg in msgs {
        println!("   {:?}", msg);
    }

    println!("\nTypedCmd provides type-safe async operations!");
    println!("Use it with the Component trait for TEA-style architecture.");
}
