use crate::components::InteractionMode;
use crate::core::{AccessibilityProps, AccessibilityRole};

pub(crate) fn command_palette_props(
    title: Option<&str>,
    query: &str,
    command_count: usize,
    mode: InteractionMode,
    open: bool,
) -> AccessibilityProps {
    let mut props = AccessibilityProps::new(AccessibilityRole::Menu)
        .label(title.unwrap_or("Command palette"))
        .description(format!("{} commands", command_count))
        .disabled(mode.is_disabled())
        .read_only(mode.is_read_only())
        .focusable(open && !mode.is_disabled());

    if !query.is_empty() {
        props = props.value(query);
    }

    props
}
