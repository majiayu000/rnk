//! Theme demo - Theming system
//!
//! Run: cargo run --example theme_demo

use rnk::components::{Theme, get_theme, set_theme, with_theme};
use rnk::core::Color;

fn main() {
    println!("=== Theme System Demo ===\n");

    // Available themes
    println!("--- Available Themes ---");
    for name in Theme::available_themes() {
        println!("  - {}", name);
    }
    println!();

    // Theme by name
    println!("--- Theme::by_name() ---");
    let theme = Theme::by_name("dark").unwrap();
    println!("  Theme::by_name(\"dark\"): {}", theme.name);

    let theme = Theme::by_name("monokai").unwrap();
    println!("  Theme::by_name(\"monokai\"): {}", theme.name);

    let result = Theme::by_name("nonexistent");
    println!(
        "  Theme::by_name(\"nonexistent\"): {:?}",
        result.map(|t| t.name)
    );
    println!();

    // Dark theme
    println!("--- Dark Theme (default) ---");
    let theme = Theme::dark();
    print_theme_colors(&theme);

    // Light theme
    println!("--- Light Theme ---");
    let theme = Theme::light();
    print_theme_colors(&theme);

    // Monokai theme
    println!("--- Monokai Theme ---");
    let theme = Theme::monokai();
    print_theme_colors(&theme);

    // Dracula theme
    println!("--- Dracula Theme ---");
    let theme = Theme::dracula();
    print_theme_colors(&theme);

    // Nord theme
    println!("--- Nord Theme ---");
    let theme = Theme::nord();
    print_theme_colors(&theme);

    // Solarized theme
    println!("--- Solarized Dark Theme ---");
    let theme = Theme::solarized_dark();
    print_theme_colors(&theme);

    // Theme builder
    println!("--- Custom Theme (ThemeBuilder) ---");
    let custom = Theme::builder("my-theme")
        .primary(Color::Rgb(255, 100, 100))
        .secondary(Color::Rgb(100, 255, 100))
        .success(Color::Rgb(100, 255, 100))
        .warning(Color::Rgb(255, 200, 100))
        .error(Color::Rgb(255, 100, 100))
        .build();

    println!("  name: {}", custom.name);
    print_color_sample("  primary", custom.primary);
    print_color_sample("  secondary", custom.secondary);
    println!();

    // Global theme context
    println!("--- Global Theme Context ---");
    println!("  get_theme().name: {}", get_theme().name);

    set_theme(Theme::light());
    println!("  set_theme(Theme::light())");
    println!("  get_theme().name: {}", get_theme().name);

    set_theme(Theme::dark());
    println!("  set_theme(Theme::dark())");
    println!("  get_theme().name: {}", get_theme().name);
    println!();

    // with_theme
    println!("--- with_theme() ---");
    println!("  Current theme: {}", get_theme().name);

    let result = with_theme(Theme::monokai(), |theme| {
        println!("  Inside with_theme: {}", theme.name);
        theme.name.clone()
    });
    println!("  Returned: {}", result);
    println!("  After with_theme: {}", get_theme().name);
    println!();

    // Visual representation
    println!("--- Visual Theme Comparison ---\n");

    let themes = [
        Theme::dark(),
        Theme::light(),
        Theme::monokai(),
        Theme::dracula(),
        Theme::nord(),
    ];

    for theme in &themes {
        print_theme_visual(theme);
    }

    // Component colors
    println!("--- Component Colors (Dark Theme) ---");
    let theme = Theme::dark();

    println!("  Input:");
    print_color_sample("    background", theme.components.input.background);
    print_color_sample("    text", theme.components.input.text);
    print_color_sample("    cursor", theme.components.input.cursor);
    println!();

    println!("  Button:");
    print_color_sample("    primary_bg", theme.components.button.primary_bg);
    print_color_sample("    primary_text", theme.components.button.primary_text);
    print_color_sample("    danger_bg", theme.components.button.danger_bg);
    println!();

    println!("  List:");
    print_color_sample("    focused_bg", theme.components.list.focused_bg);
    print_color_sample("    selected_bg", theme.components.list.selected_bg);
    println!();

    println!("  Progress:");
    print_color_sample("    track", theme.components.progress.track);
    print_color_sample("    fill", theme.components.progress.fill);
    println!();

    // Usage example
    println!("--- Usage in TUI App ---");
    println!("```rust");
    println!("use rnk::components::{{Theme, get_theme, set_theme}};");
    println!("use rnk::hooks::use_signal;");
    println!();
    println!("fn themed_app() -> Element {{");
    println!("    let theme_name = use_signal(|| \"dark\".to_string());");
    println!();
    println!("    // Switch theme");
    println!("    let switch_theme = move |name: &str| {{");
    println!("        if let Some(theme) = Theme::by_name(name) {{");
    println!("            set_theme(theme);");
    println!("            theme_name.set(name.to_string());");
    println!("        }}");
    println!("    }};");
    println!();
    println!("    let theme = get_theme();");
    println!();
    println!("    // Use theme colors in components");
    println!("    col![");
    println!("        Text::new(\"My App\")");
    println!("            .color(theme.primary),");
    println!("        Text::new(\"Success!\")");
    println!("            .color(theme.success),");
    println!("        Text::new(\"Warning!\")");
    println!("            .color(theme.warning),");
    println!("    ].into_element()");
    println!("}}");
    println!("```");
}

fn print_theme_colors(theme: &Theme) {
    println!("  name: {}", theme.name);
    print_color_sample("  primary", theme.primary);
    print_color_sample("  secondary", theme.secondary);
    print_color_sample("  success", theme.success);
    print_color_sample("  warning", theme.warning);
    print_color_sample("  error", theme.error);
    print_color_sample("  info", theme.info);
    println!();
}

fn print_color_sample(label: &str, color: Color) {
    let ansi = color_to_ansi_fg(&color);
    println!("{}: {}████\x1b[0m {:?}", label, ansi, color);
}

fn print_theme_visual(theme: &Theme) {
    println!("  {} Theme:", theme.name);
    print!("    {}Primary\x1b[0m  ", color_to_ansi_fg(&theme.primary));
    print!("{}Success\x1b[0m  ", color_to_ansi_fg(&theme.success));
    print!("{}Warning\x1b[0m  ", color_to_ansi_fg(&theme.warning));
    print!("{}Error\x1b[0m  ", color_to_ansi_fg(&theme.error));
    println!("{}Info\x1b[0m", color_to_ansi_fg(&theme.info));
    println!();
}

/// Convert a Color to ANSI foreground escape code
fn color_to_ansi_fg(color: &Color) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
        Color::Ansi256(code) => format!("\x1b[38;5;{}m", code),
        Color::Reset => "\x1b[0m".to_string(),
        Color::Black => "\x1b[30m".to_string(),
        Color::Red => "\x1b[31m".to_string(),
        Color::Green => "\x1b[32m".to_string(),
        Color::Yellow => "\x1b[33m".to_string(),
        Color::Blue => "\x1b[34m".to_string(),
        Color::Magenta => "\x1b[35m".to_string(),
        Color::Cyan => "\x1b[36m".to_string(),
        Color::White => "\x1b[37m".to_string(),
        Color::BrightBlack => "\x1b[90m".to_string(),
        Color::BrightRed => "\x1b[91m".to_string(),
        Color::BrightGreen => "\x1b[92m".to_string(),
        Color::BrightYellow => "\x1b[93m".to_string(),
        Color::BrightBlue => "\x1b[94m".to_string(),
        Color::BrightMagenta => "\x1b[95m".to_string(),
        Color::BrightCyan => "\x1b[96m".to_string(),
        Color::BrightWhite => "\x1b[97m".to_string(),
    }
}
