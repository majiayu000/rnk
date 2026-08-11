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
//! Wrapping comes from `ComposerProjection`; the example never counts bytes,
//! characters, or terminal cells itself.
//!
//! Run with: cargo run --example chat

use rnk::components::InteractionOutcome;
use rnk::components::chat::{
    BlockId, ChatComposerKeyMap, ChatComposerState, ChatMessage, ChatMessageView, ChatRole,
    ComposerProjection, ConversationError, ConversationEvent, ConversationGuard, ConversationState,
    ConversationUpdate, MessageBlock, MessageBlockEntry, MessageId, MessageMutationGuard, UpdateId,
    handle_key,
};
use rnk::prelude::*;
use std::num::NonZeroUsize;

/// Columns the input line is laid out in.
const INPUT_WIDTH: u16 = 60;

fn main() -> std::io::Result<()> {
    render(app).run()
}

pub(crate) fn app() -> Element {
    let app = use_app();
    let composer = use_signal(ChatComposerState::new);
    let conversation = use_signal(|| ConversationState::new(0, NonZeroUsize::MIN));
    let status = use_signal(|| None::<String>);

    let composer_for_handler = composer.clone();
    let conversation_for_handler = conversation.clone();
    let status_for_handler = status.clone();

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
            if let Some(token) = state.pending_submission().map(|pending| pending.token()) {
                let mut candidate = conversation_for_handler.get();
                match append_user_message(&mut candidate, text) {
                    Ok(()) => match state.acknowledge_success(token) {
                        Ok(()) => {
                            conversation_for_handler.set(candidate);
                            status_for_handler.set(None);
                        }
                        Err(error) => {
                            status_for_handler.set(Some(format!("composer failure: {error:?}")))
                        }
                    },
                    Err(error) => {
                        let acknowledgement = state.acknowledge_failure(token);
                        status_for_handler.set(Some(match acknowledgement {
                            Ok(()) => error.to_string(),
                            Err(acknowledgement) => {
                                format!(
                                    "{error}; composer acknowledgement failed: {acknowledgement:?}"
                                )
                            }
                        }));
                    }
                }
            }
        }

        composer_for_handler.set(state);
    });

    let current_conversation = conversation.get();
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
                    current_conversation
                        .messages()
                        .iter()
                        .map(|message| ChatMessageView::new(message).into_element()),
                )
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(render_input(&projection))
        .child(status.get().map_or_else(
            || Text::new("").into_element(),
            |error| Text::new(error).color(Color::Red).into_element(),
        ))
        .child(Newline::new().into_element())
        .child(
            Text::new("Press Enter to send, Ctrl+Q to quit")
                .dim()
                .into_element(),
        )
        .into_element()
}

fn append_user_message(
    state: &mut ConversationState,
    text: String,
) -> Result<(), ConversationError> {
    let identity = state.expected_sequence();
    let message_id = MessageId::new(identity);
    let block_id = BlockId::new(identity);
    let message = ChatMessage::new(
        message_id,
        ChatRole::User,
        vec![MessageBlockEntry::new(block_id, MessageBlock::Text(text))],
    )?;
    state.apply_event(ConversationEvent::new(
        UpdateId::new(format!("chat-push-{identity}"))?,
        state.expected_sequence(),
        ConversationUpdate::push(ConversationGuard::new(state.revision()), message),
    ))?;

    let message = state
        .message(message_id)
        .ok_or(ConversationError::UnknownMessage { message_id })?;
    let guard = MessageMutationGuard::new(
        ConversationGuard::new(state.revision()),
        message_id,
        message.revision(),
    );
    state.apply_event(ConversationEvent::new(
        UpdateId::new(format!("chat-complete-{identity}"))?,
        state.expected_sequence(),
        ConversationUpdate::complete(guard),
    ))?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn gh68_offline_adapter_view() -> Result<String, ConversationError> {
    let mut state = ConversationState::new(0, NonZeroUsize::new(16).unwrap());
    append_user_message(&mut state, "Explain the release gate".to_owned())?;
    let identity = state.expected_sequence();
    let message_id = MessageId::new(identity);
    let message = ChatMessage::new(
        message_id,
        ChatRole::Assistant,
        vec![MessageBlockEntry::new(
            BlockId::new(identity),
            MessageBlock::Text("Use typed updates.".to_owned()),
        )],
    )?;
    state.apply_event(ConversationEvent::new(
        UpdateId::new("offline-assistant")?,
        state.expected_sequence(),
        ConversationUpdate::push(ConversationGuard::new(state.revision()), message),
    ))?;
    let guard = MessageMutationGuard::new(
        ConversationGuard::new(state.revision()),
        message_id,
        state
            .message(message_id)
            .ok_or(ConversationError::UnknownMessage { message_id })?
            .revision(),
    );
    state.apply_event(ConversationEvent::new(
        UpdateId::new("offline-complete")?,
        state.expected_sequence(),
        ConversationUpdate::complete(guard),
    ))?;
    let root = Box::new()
        .flex_direction(FlexDirection::Column)
        .children(
            state
                .messages()
                .iter()
                .map(|message| ChatMessageView::new(message).into_element()),
        )
        .into_element();
    Ok(rnk::render_to_string(&root, 60))
}

/// Draws the exact rows projected by the shared composer.
fn render_input(projection: &ComposerProjection) -> Element {
    Box::new()
        .flex_direction(FlexDirection::Row)
        .child(
            Text::new("Enter message: ")
                .color(Color::Green)
                .into_element(),
        )
        .child(Text::new(projection.visible_slice().join("\n")).into_element())
        .child(Text::new("▏").color(Color::BrightCyan).into_element())
        .into_element()
}
