//! Paginator demo - Page navigation component
//!
//! Run: cargo run --example paginator_demo

use rnk::components::{Paginator, PaginatorState, PaginatorStyle, PaginatorType};

fn main() {
    println!("=== Paginator Component Demo ===\n");

    // Basic paginator state
    println!("--- PaginatorState API ---");
    let mut state = PaginatorState::new(10);
    println!("PaginatorState::new(10):");
    println!(
        "  page: {} (display: {})",
        state.page(),
        state.page_display()
    );
    println!("  total_pages: {}", state.total_pages());
    println!("  on_first_page: {}", state.on_first_page());
    println!("  on_last_page: {}", state.on_last_page());
    println!("  progress: {:.0}%", state.progress() * 100.0);
    println!();

    // Navigation
    println!("--- Navigation ---");
    state.next_page();
    println!("next_page(): page = {}", state.page_display());

    state.next_page();
    state.next_page();
    println!("next_page() x2: page = {}", state.page_display());

    state.prev_page();
    println!("prev_page(): page = {}", state.page_display());

    state.last_page();
    println!(
        "last_page(): page = {}, on_last_page = {}",
        state.page_display(),
        state.on_last_page()
    );

    state.first_page();
    println!(
        "first_page(): page = {}, on_first_page = {}",
        state.page_display(),
        state.on_first_page()
    );
    println!();

    // From items
    println!("--- From Items ---");
    let state = PaginatorState::from_items(95, 10);
    println!("PaginatorState::from_items(95, 10):");
    println!(
        "  total_pages: {} (95 items / 10 per page)",
        state.total_pages()
    );
    println!("  per_page: {}", state.per_page());
    println!();

    // Slice bounds
    println!("--- Slice Bounds ---");
    let items: Vec<i32> = (1..=95).collect();
    let mut state = PaginatorState::from_items(items.len(), 10);

    println!("Page 1: slice_bounds = {:?}", state.slice_bounds());
    println!("  items: {:?}", state.page_items(&items));

    state.set_page(4);
    println!("Page 5: slice_bounds = {:?}", state.slice_bounds());
    println!("  items: {:?}", state.page_items(&items));

    state.last_page();
    println!(
        "Page {}: slice_bounds = {:?}",
        state.page_display(),
        state.slice_bounds()
    );
    println!("  items: {:?}", state.page_items(&items));
    println!();

    // Display types
    println!("--- Display Types ---");
    println!();

    println!("PaginatorType::Dots (default):");
    for page in 0..5 {
        let paginator = Paginator::new(page, 5).dots();
        println!("  Page {}: {}", page + 1, paginator.render());
    }
    println!();

    println!("PaginatorType::Arabic:");
    for page in 0..5 {
        let paginator = Paginator::new(page, 5).arabic();
        println!("  Page {}: {}", page + 1, paginator.render());
    }
    println!();

    // Style presets
    println!("--- Style Presets ---");
    let page = 2;
    let total = 5;

    println!(
        "circles() (default): {}",
        Paginator::new(page, total)
            .style(PaginatorStyle::circles())
            .render()
    );
    println!(
        "squares():           {}",
        Paginator::new(page, total)
            .style(PaginatorStyle::squares())
            .render()
    );
    println!(
        "dashes():            {}",
        Paginator::new(page, total)
            .style(PaginatorStyle::dashes())
            .render()
    );
    println!(
        "blocks():            {}",
        Paginator::new(page, total)
            .style(PaginatorStyle::blocks())
            .render()
    );
    println!();

    // Custom style
    println!("--- Custom Style ---");
    let style = PaginatorStyle::new()
        .active_dot("◆")
        .inactive_dot("◇")
        .dot_separator(" ");
    println!(
        "Custom diamonds: {}",
        Paginator::new(2, 5).style(style).render()
    );

    let style = PaginatorStyle::new().arabic_format("Page {} of {}");
    println!(
        "Custom format: {}",
        Paginator::new(2, 5).arabic().style(style).render()
    );
    println!();

    // Max dots (for many pages)
    println!("--- Max Dots (for many pages) ---");
    println!("20 pages, max_dots(7):");
    for page in [0, 5, 10, 15, 19] {
        let paginator = Paginator::new(page, 20).dots().max_dots(7);
        println!("  Page {:2}: {}", page + 1, paginator.render());
    }
    println!();

    // Visual representation with colors
    println!("--- Visual Representation ---");
    println!("  (what it would look like in a TUI app)\n");

    let page = 3;
    let total = 7;

    // Dots with color
    print!("  Dots:   ");
    for i in 0..total {
        if i == page {
            print!("\x1b[36m●\x1b[0m ");
        } else {
            print!("\x1b[90m○\x1b[0m ");
        }
    }
    println!();

    // Arabic with color
    println!("  Arabic: \x1b[36m{}/{}\x1b[0m", page + 1, total);

    // Blocks
    print!("  Blocks: ");
    for i in 0..total {
        if i == page {
            print!("\x1b[36m█\x1b[0m");
        } else {
            print!("\x1b[90m░\x1b[0m");
        }
    }
    println!();

    // Progress bar style
    print!("  Bar:    [");
    let bar_width = 20;
    let filled = ((page as f64 / (total - 1) as f64) * bar_width as f64) as usize;
    for i in 0..bar_width {
        if i <= filled {
            print!("\x1b[36m━\x1b[0m");
        } else {
            print!("\x1b[90m─\x1b[0m");
        }
    }
    println!("] {:.0}%", (page as f64 / (total - 1) as f64) * 100.0);
    println!();

    // Usage example
    println!("--- Usage in TUI App ---");
    println!("```rust");
    println!("use rnk::components::{{Paginator, PaginatorState, handle_paginator_input}};");
    println!("use rnk::hooks::{{use_signal, use_input}};");
    println!();
    println!("fn app() -> Element {{");
    println!("    let state = use_signal(|| PaginatorState::from_items(100, 10));");
    println!();
    println!("    use_input(move |input, key| {{");
    println!("        let mut s = state.get();");
    println!("        if handle_paginator_input(&mut s, input, key) {{");
    println!("            state.set(s);");
    println!("        }}");
    println!("    }});");
    println!();
    println!("    let s = state.get();");
    println!("    let items = get_all_items();");
    println!("    let page_items = s.page_items(&items);");
    println!();
    println!("    Box::new()");
    println!("        .child(render_items(page_items))");
    println!("        .child(Paginator::from_state(&s).dots().into_element())");
    println!("        .into_element()");
    println!("}}");
    println!("```");
}
