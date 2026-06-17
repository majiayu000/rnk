use super::*;

#[test]
fn test_theme_dark() {
    let theme = Theme::dark();
    assert_eq!(theme.name, "dark");
    assert_eq!(theme.primary, Color::Cyan);
}

#[test]
fn test_theme_light() {
    let theme = Theme::light();
    assert_eq!(theme.name, "light");
    assert_eq!(theme.primary, Color::Blue);
}

#[test]
fn test_theme_monokai() {
    let theme = Theme::monokai();
    assert_eq!(theme.name, "monokai");
}

#[test]
fn test_theme_dracula() {
    let theme = Theme::dracula();
    assert_eq!(theme.name, "dracula");
}

#[test]
fn test_theme_nord() {
    let theme = Theme::nord();
    assert_eq!(theme.name, "nord");
}

#[test]
fn test_theme_solarized() {
    let theme = Theme::solarized_dark();
    assert_eq!(theme.name, "solarized_dark");
}

#[test]
fn test_theme_by_name() {
    assert!(Theme::by_name("dark").is_some());
    assert!(Theme::by_name("light").is_some());
    assert!(Theme::by_name("monokai").is_some());
    assert!(Theme::by_name("dracula").is_some());
    assert!(Theme::by_name("nord").is_some());
    assert!(Theme::by_name("solarized").is_some());
    assert!(Theme::by_name("nonexistent").is_none());
}

#[test]
fn test_available_themes() {
    let themes = Theme::available_themes();
    assert!(themes.contains(&"dark"));
    assert!(themes.contains(&"light"));
    assert!(themes.contains(&"monokai"));
}

#[test]
fn test_theme_builder() {
    let theme = Theme::builder("custom")
        .primary(Color::Red)
        .secondary(Color::Blue)
        .success(Color::Green)
        .build();

    assert_eq!(theme.name, "custom");
    assert_eq!(theme.primary, Color::Red);
    assert_eq!(theme.secondary, Color::Blue);
    assert_eq!(theme.success, Color::Green);
}

#[test]
fn test_set_get_theme() {
    let original = get_theme();

    set_theme(Theme::light());
    assert_eq!(get_theme().name, "light");

    set_theme(Theme::dark());
    assert_eq!(get_theme().name, "dark");

    set_theme(original);
}

#[test]
fn test_with_theme() {
    let result = with_theme(Theme::monokai(), |theme| {
        assert_eq!(theme.name, "monokai");
        42
    });
    assert_eq!(result, 42);
}

#[test]
fn test_with_theme_restores_after_panic() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let original = get_theme();
    let result = catch_unwind(AssertUnwindSafe(|| {
        with_theme(Theme::light(), |_| {
            panic!("boom");
        });
    }));

    assert!(result.is_err());
    assert_eq!(get_theme().name, original.name);
}

#[test]
fn test_theme_isolated_per_runtime_context() {
    use crate::runtime::{RuntimeContext, set_current_runtime};
    use std::cell::RefCell;
    use std::rc::Rc;

    set_current_runtime(None);
    set_theme(Theme::dark());
    assert_eq!(get_theme().name, "dark");

    let ctx1 = Rc::new(RefCell::new(RuntimeContext::new()));
    let ctx2 = Rc::new(RefCell::new(RuntimeContext::new()));

    set_current_runtime(Some(ctx1));
    set_theme(Theme::light());
    assert_eq!(get_theme().name, "light");

    set_current_runtime(Some(ctx2));
    assert_eq!(get_theme().name, "dark");

    set_current_runtime(None);
    assert_eq!(get_theme().name, "dark");
}

#[test]
fn test_semantic_color() {
    let theme = Theme::dark();

    assert_eq!(theme.semantic_color(SemanticColor::Primary), theme.primary);
    assert_eq!(
        theme.semantic_color(SemanticColor::Secondary),
        theme.secondary
    );
    assert_eq!(theme.semantic_color(SemanticColor::Success), theme.success);
    assert_eq!(theme.semantic_color(SemanticColor::Warning), theme.warning);
    assert_eq!(theme.semantic_color(SemanticColor::Error), theme.error);
    assert_eq!(theme.semantic_color(SemanticColor::Info), theme.info);
    assert_eq!(
        theme.semantic_color(SemanticColor::TextPrimary),
        theme.text.primary
    );
    assert_eq!(
        theme.semantic_color(SemanticColor::Background),
        theme.background.default
    );
    assert_eq!(
        theme.semantic_color(SemanticColor::Border),
        theme.border.default
    );
}

#[test]
fn test_semantic_fg_bg() {
    let theme = Theme::dark();

    let fg = theme.semantic_fg(SemanticColor::Error);
    assert!(fg.starts_with("\x1b["));

    let bg = theme.semantic_bg(SemanticColor::Error);
    assert!(bg.starts_with("\x1b["));
}
