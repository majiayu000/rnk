//! Chat example - the smallest complete chat surface.
//!
//! Equivalent to ink's examples/chat.
//!
//! The input is `ChatComposerState`, not a `String`. That matters more than it
//! looks: this example used to backspace with `String::pop`, which removes one
//! `char`. A `char` is not a user-perceived character — deleting from "café"
//! written with a combining accent, or from an emoji built out of a ZWJ
//! sequence, takes a piece off the end and leaves a different character behind.
//! The composer deletes by grapheme cluster, so backspace removes what the user
//! sees.
//!
//! Wrapping and the cursor's visual position come from `ComposerProjection` for
//! the same reason: a CJK character is two cells wide, so counting characters
//! puts the cursor in the wrong column as soon as one appears.
//!
//! Run with: cargo run --example chat

use rnk::components::InteractionOutcome;
use rnk::components::chat::{
    ChatComposerKeyMap, ChatComposerState, ComposerProjection, handle_key,
};
use rnk::prelude::*;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Columns the input line is laid out in.
const INPUT_WIDTH: u16 = 60;

fn main() -> std::io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let app = use_app();
    let composer = use_signal(ChatComposerState::new);
    let messages = use_signal(Vec::<String>::new);

    let composer_for_handler = composer.clone();
    let messages_for_handler = messages.clone();

    use_input(move |input, key| {
        if input == "q" && key.ctrl {
            app.exit();
            return;
        }

        // One call covers typing, grapheme-aware deletion, cursor movement and
        // submission. The example does not decide what any key means.
        let mut state = composer_for_handler.get();
        let outcome = handle_key(&mut state, &ChatComposerKeyMap::new(), input, key);

        if let InteractionOutcome::Submitted(text) = outcome {
            messages_for_handler.update(|messages| messages.push(format!("You: {text}")));

            // Enter stages a submission; it does not clear the draft. Clearing
            // happens here, once the send is known to have succeeded — a failed
            // send would call `acknowledge_failure` and leave the text in place
            // rather than destroy it at the moment it is hardest to retype.
            if let Some(token) = state.pending_submission().map(|pending| pending.token()) {
                let _ = state.acknowledge_success(token);
            }
        }

        composer_for_handler.set(state);
    });

    let current_messages = messages.get();
    let projection = ComposerProjection::build(&composer.get(), INPUT_WIDTH);

    Box::new()
        .flex_direction(FlexDirection::Column)
        .padding(1)
        .child(
            Box::new()
                .border_style(BorderStyle::Round)
                .border_color(Color::Cyan)
                .padding_x(2.0)
                .child(
                    Text::new("rnk Chat")
                        .color(Color::Cyan)
                        .bold()
                        .into_element(),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .min_height(10.0)
                .children(
                    current_messages
                        .iter()
                        .map(|message| Text::new(message).color(Color::White).into_element()),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(render_input(&projection))
        .child(Newline::new().into_element())
        .child(
            Text::new("Press Enter to send, Ctrl+Q to quit")
                .dim()
                .into_element(),
        )
        .into_element()
}

/// Draws the draft's visible rows, with the cursor on the row that holds it.
fn render_input(projection: &ComposerProjection) -> Element {
    let first_row = projection.scroll_offset();
    let mut container = Box::new().flex_direction(FlexDirection::Column);

    for (offset, row) in projection.visible_slice().iter().enumerate() {
        let absolute_row = first_row + offset;
        let prefix = if offset == 0 {
            "Enter message: "
        } else {
            "               "
        };
        // The cursor's column comes from the projection, in terminal cells. A
        // character count would land in the wrong place after the first CJK
        // character, which occupies two.
        let cursor_column =
            (absolute_row == projection.cursor_row()).then(|| projection.cursor_column());

        let mut line = Box::new()
            .flex_direction(FlexDirection::Row)
            .child(Text::new(prefix).color(Color::Green).into_element());

        let mut column = 0usize;
        let mut painted_cursor = false;
        for cluster in row.graphemes(true) {
            let at_cursor = cursor_column == Some(column);
            painted_cursor |= at_cursor;
            line = line.child(cell(cluster, at_cursor));
            // Advance by cells, not clusters: a CJK character is two columns.
            column += UnicodeWidthStr::width(cluster).max(1);
        }
        // A cursor past the last cluster has no character to sit on, so it gets
        // a space of its own rather than disappearing.
        if let Some(target) = cursor_column
            && !painted_cursor
            && target >= column
        {
            line = line.child(cell(" ", true));
        }

        container = container.child(line.into_element());
    }

    container.into_element()
}

fn cell(cluster: &str, at_cursor: bool) -> Element {
    let text = Text::new(cluster.to_string());
    if at_cursor {
        text.color(Color::Black)
            .background(Color::BrightCyan)
            .into_element()
    } else {
        text.color(Color::White).into_element()
    }
}
