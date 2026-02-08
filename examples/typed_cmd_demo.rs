//! Typed Command Demo - Type-safe message passing.
//!
//! This example demonstrates `Cmd<M>` for type-safe async operations
//! that return typed messages.

use rnk::cmd::{AppMsg, Cmd};

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
fn load_data() -> Cmd<Msg> {
    Cmd::perform(|| async {
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
fn start_timer(count: u64) -> Cmd<Msg> {
    Cmd::tick(std::time::Duration::from_secs(1), move |_| {
        Msg::Tick(count + 1)
    })
}

/// Batch multiple commands
fn init_commands() -> Cmd<Msg> {
    Cmd::batch(vec![load_data(), start_timer(0)])
}

/// Map child messages to parent messages
#[derive(Debug)]
enum ParentMsg {
    Child(Msg),
    Other,
}

fn map_example() -> Cmd<ParentMsg> {
    // Child command produces Msg
    let child_cmd: Cmd<Msg> = load_data();

    // Map to parent message type
    child_cmd.map(ParentMsg::Child)
}

fn main() {
    println!("Typed Command Demo - Type-safe message passing\n");

    // Demonstrate creating various typed commands
    println!("1. Creating a perform command:");
    let cmd: Cmd<Msg> = load_data();
    println!("   {:?}\n", cmd);

    println!("2. Creating a tick command:");
    let cmd: Cmd<Msg> = start_timer(0);
    println!("   {:?}\n", cmd);

    println!("3. Batching commands:");
    let cmd: Cmd<Msg> = init_commands();
    println!("   {:?}\n", cmd);

    println!("4. Mapping message types:");
    let cmd: Cmd<ParentMsg> = map_example();
    println!("   {:?}\n", cmd);

    println!("5. Sequencing commands:");
    let cmd: Cmd<Msg> = Cmd::sequence(vec![
        Cmd::sleep(std::time::Duration::from_millis(100)),
        load_data(),
    ]);
    println!("   {:?}\n", cmd);

    println!("6. Chaining with and_then:");
    let cmd: Cmd<Msg> = Cmd::sleep(std::time::Duration::from_millis(50)).and_then(load_data());
    println!("   {:?}\n", cmd);

    // Show AppMsg usage
    println!("7. Built-in AppMsg types:");
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

    println!("\nCmd<M> provides type-safe async operations!");
    println!("Use it with the Component trait for TEA-style architecture.");
}
