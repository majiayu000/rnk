//! Shared list rendering helpers for selection components.

use crate::components::navigation::calculate_visible_range;
use crate::components::{Box as TinkBox, Text};
use crate::core::{Color, Element, FlexDirection};

/// Shared style accessors for selectable lists.
pub(crate) trait ListStyle {
    fn highlight_color(&self) -> Option<Color>;
    fn highlight_bg(&self) -> Option<Color>;
    fn highlight_bold(&self) -> bool;
    fn indicator(&self) -> &str;
    fn indicator_padding(&self) -> &str;
    fn item_color(&self) -> Option<Color>;
}

/// Compute indicator padding based on indicator width.
pub(crate) fn indicator_padding(indicator: &str) -> String {
    " ".repeat(indicator.chars().count())
}

/// Render a selectable list with shared highlight handling.
pub(crate) fn render_list<T, S, F, G>(
    items: &[T],
    highlighted: usize,
    limit: Option<usize>,
    style: &S,
    label_builder: F,
    decorate: G,
) -> Element
where
    S: ListStyle,
    F: Fn(&T, usize, bool, &str) -> String,
    G: Fn(&T, usize, &S, bool, Text) -> Text,
{
    let total_items = items.len();
    let (start, end) = calculate_visible_range(highlighted, total_items, limit);

    let mut container = TinkBox::new().flex_direction(FlexDirection::Column);

    for idx in start..end {
        let item = &items[idx];
        let is_highlighted = idx == highlighted;

        let prefix = if is_highlighted {
            style.indicator()
        } else {
            style.indicator_padding()
        };

        let label = label_builder(item, idx, is_highlighted, prefix);
        let mut text = Text::new(&label);

        if is_highlighted {
            if let Some(color) = style.highlight_color() {
                text = text.color(color);
            }
            if let Some(bg) = style.highlight_bg() {
                text = text.background(bg);
            }
            if style.highlight_bold() {
                text = text.bold();
            }
        } else {
            text = decorate(item, idx, style, is_highlighted, text);
        }

        container = container.child(text.into_element());
    }

    container.into_element()
}
