//! Confirm demo - Yes/No confirmation dialog
//!
//! Run: cargo run --example confirm_demo

use rnk::components::{ButtonStyle, Confirm, ConfirmState, ConfirmStyle};

fn main() {
    println!("=== Confirm Component Demo ===\n");

    // Basic confirm state
    println!("--- ConfirmState API ---");
    let mut state = ConfirmState::new("Delete this file?");
    println!("ConfirmState::new(\"Delete this file?\"):");
    println!("  prompt: {:?}", state.prompt());
    println!("  is_yes_focused: {}", state.is_yes_focused());
    println!("  is_no_focused: {}", state.is_no_focused());
    println!("  is_answered: {}", state.is_answered());
    println!();

    // Default yes
    let state_yes = ConfirmState::default_yes("Continue?");
    println!("ConfirmState::default_yes(\"Continue?\"):");
    println!("  is_yes_focused: {}", state_yes.is_yes_focused());
    println!();

    // Toggle focus
    println!("--- Focus Navigation ---");
    println!("Initial: is_no_focused = {}", state.is_no_focused());
    state.toggle_focus();
    println!(
        "toggle_focus(): is_yes_focused = {}",
        state.is_yes_focused()
    );
    state.toggle_focus();
    println!("toggle_focus(): is_no_focused = {}", state.is_no_focused());
    println!();

    // Confirm/Cancel
    println!("--- Confirm/Cancel ---");
    let mut state = ConfirmState::new("Test?");
    state.confirm();
    println!(
        "confirm(): result = {:?}, is_confirmed = {}",
        state.result(),
        state.is_confirmed()
    );

    let mut state = ConfirmState::new("Test?");
    state.cancel();
    println!(
        "cancel(): result = {:?}, is_cancelled = {}",
        state.result(),
        state.is_cancelled()
    );
    println!();

    // Submit (based on focus)
    println!("--- Submit (based on focus) ---");
    let mut state = ConfirmState::new("Test?");
    state.focus_yes();
    state.submit();
    println!(
        "focus_yes() + submit(): is_confirmed = {}",
        state.is_confirmed()
    );

    let mut state = ConfirmState::new("Test?");
    state.focus_no();
    state.submit();
    println!(
        "focus_no() + submit(): is_cancelled = {}",
        state.is_cancelled()
    );
    println!();

    // Reset
    println!("--- Reset ---");
    let mut state = ConfirmState::default_yes("Test?");
    state.focus_no();
    state.confirm();
    println!("After confirm: is_answered = {}", state.is_answered());
    state.reset();
    println!(
        "After reset: is_answered = {}, is_yes_focused = {}",
        state.is_answered(),
        state.is_yes_focused()
    );
    println!();

    // Style presets
    println!("--- Style Presets ---");
    println!();

    let presets = [
        ("default()", ConfirmStyle::default()),
        ("confirm_cancel()", ConfirmStyle::confirm_cancel()),
        ("ok_cancel()", ConfirmStyle::ok_cancel()),
        ("save_discard()", ConfirmStyle::save_discard()),
        ("delete_keep()", ConfirmStyle::delete_keep()),
    ];

    for (name, style) in presets {
        let state = ConfirmState::new("Action?");
        let confirm = Confirm::new(&state).style(style);
        println!("ConfirmStyle::{}:", name);
        println!("  {}", confirm.render());
    }
    println!();

    // Button styles
    println!("--- Button Styles ---");
    let button_styles = [
        ("Brackets", ButtonStyle::Brackets),
        ("Angles", ButtonStyle::Angles),
        ("Parens", ButtonStyle::Parens),
        ("Plain", ButtonStyle::Plain),
        ("Padded", ButtonStyle::Padded),
    ];

    for (name, btn_style) in button_styles {
        let state = ConfirmState::new("Delete?");
        let style = ConfirmStyle::default().button_style(btn_style);
        let confirm = Confirm::new(&state).style(style);
        println!("  {}: {}", name, confirm.render());
    }
    println!();

    // Visual representation
    println!("--- Visual Representation ---");
    println!("  (what it would look like in a TUI app)\n");

    // Focused on No
    let state = ConfirmState::new("Delete this file?");
    print_confirm_visual(&state, true);

    // Focused on Yes
    let mut state = ConfirmState::new("Delete this file?");
    state.focus_yes();
    print_confirm_visual(&state, true);

    // Different styles
    println!("\n  Different prompt styles:");
    println!();

    // Danger style
    print!("  \x1b[31mDelete this file permanently?\x1b[0m ");
    println!("\x1b[90m[Yes](Y)\x1b[0m  \x1b[1;37;41m [No](N) \x1b[0m");

    // Info style
    print!("  \x1b[36mSave changes before closing?\x1b[0m ");
    println!("\x1b[1;37;46m [Save](Y) \x1b[0m  \x1b[90m[Discard](N)\x1b[0m");

    println!();

    // Keyboard hints
    println!("--- Keyboard Shortcuts ---");
    println!("  Tab / ←/→  - Toggle between Yes/No");
    println!("  Enter/Space - Submit focused option");
    println!("  Y          - Confirm (Yes)");
    println!("  N          - Cancel (No)");
    println!("  Escape     - Cancel");
    println!();

    // Usage example
    println!("--- Usage in TUI App ---");
    println!("```rust");
    println!("use rnk::components::{{Confirm, ConfirmState, handle_confirm_input}};");
    println!("use rnk::hooks::{{use_signal, use_input}};");
    println!();
    println!("fn app() -> Element {{");
    println!("    let state = use_signal(|| ConfirmState::new(\"Delete this file?\"));");
    println!();
    println!("    use_input(move |input, key| {{");
    println!("        let mut s = state.get();");
    println!("        if handle_confirm_input(&mut s, input, key) {{");
    println!("            state.set(s);");
    println!("        }}");
    println!("    }});");
    println!();
    println!("    let s = state.get();");
    println!("    if let Some(confirmed) = s.result() {{");
    println!("        if confirmed {{");
    println!("            return Text::new(\"File deleted!\").into_element();");
    println!("        }} else {{");
    println!("            return Text::new(\"Cancelled.\").into_element();");
    println!("        }}");
    println!("    }}");
    println!();
    println!("    Confirm::new(&s)");
    println!("        .style(ConfirmStyle::delete_keep())");
    println!("        .into_element()");
    println!("}}");
    println!("```");
}

fn print_confirm_visual(state: &ConfirmState, focused: bool) {
    let yes_focused = state.is_yes_focused();

    print!("  {} ", state.prompt());

    if yes_focused && focused {
        print!("\x1b[1;37;46m [Yes](Y) \x1b[0m");
    } else {
        print!("\x1b[90m[Yes](Y)\x1b[0m");
    }

    print!("  ");

    if !yes_focused && focused {
        print!("\x1b[1;37;46m [No](N) \x1b[0m");
    } else {
        print!("\x1b[90m[No](N)\x1b[0m");
    }

    println!();
}
