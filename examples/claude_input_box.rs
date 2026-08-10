//! Claude-style inline input backed by the public scrollback lifecycle.
//!
//! Earlier versions maintained a second `println` ledger and cleared the draft
//! after any write attempt. This focused example deliberately removes the
//! bespoke opening animation and demonstrates the durable boundary instead:
//! only [`InlineCommitReport::Fixed`] acknowledges the composer submission.
//! `Retained` keeps the draft retryable and `Latched` remains blocked until an
//! application obtains a human [`UnknownResolution`].
//!
//! Run with: `cargo run --example claude_input_box`

use std::io;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use rnk::components::chat::scrollback::NativeTerminalSink;
use rnk::components::chat::{
    ChatComposerKeyMap, ComposerProjection, InlineChatShell, InlineCommitReport, InlineKeyOutcome,
    LiveState, MessageId, MessageRevision, ProjectionContext, ScrollbackNamespace, ThemeIdentity,
};
use rnk::prelude::*;

const MAX_VISIBLE_INPUT_LINES: NonZeroUsize = NonZeroUsize::new(4).expect("four is non-zero");
type ExampleShell = InlineChatShell<NativeTerminalSink<io::Stdout>>;

fn main() -> io::Result<()> {
    render(app).run()
}

fn app() -> Element {
    let app = use_app();
    let shell = use_signal(|| {
        let namespace = ScrollbackNamespace::new("example.claude-input")
            .expect("the example namespace is non-empty and restart-stable");
        let mut shell = InlineChatShell::new(namespace, NativeTerminalSink::new(io::stdout()));
        *shell.composer_mut() = shell
            .composer()
            .clone()
            .with_max_visible_lines(MAX_VISIBLE_INPUT_LINES);
        Arc::new(Mutex::new(shell))
    });
    let status = use_signal(|| "ready".to_owned());
    let committed = use_signal(|| 0_u64);

    let input_shell = shell.clone();
    let input_status = status.clone();
    let input_committed = committed.clone();
    use_input(move |input, key| {
        if key.escape || (key.ctrl && input.eq_ignore_ascii_case("c")) {
            app.exit();
            return;
        }
        let width = match rnk::renderer::Terminal::size() {
            Ok((0, _)) => {
                input_status.set("terminal width is zero".to_owned());
                return;
            }
            Ok((width, _)) => width,
            Err(error) => {
                input_status.set(format!("terminal size unavailable: {error}"));
                return;
            }
        };
        let handle = input_shell.get();
        let mut shell = match handle.lock() {
            Ok(shell) => shell,
            Err(_) => {
                input_status.set("inline shell lock is poisoned".to_owned());
                return;
            }
        };
        match shell.handle_key(&ChatComposerKeyMap::new(), input, key) {
            InlineKeyOutcome::Submitted(text) => {
                commit_submission(&mut shell, text, width, &input_status, &input_committed)
            }
            InlineKeyOutcome::Cancelled => input_status.set("input cancelled".to_owned()),
            InlineKeyOutcome::Changed(_) => input_status.set("editing".to_owned()),
            InlineKeyOutcome::Handled => input_status.set("input handled".to_owned()),
            InlineKeyOutcome::Ignored => input_status.set("key ignored".to_owned()),
            InlineKeyOutcome::NotFocused => input_status.set("composer is not focused".to_owned()),
        }
    });

    let handle = shell.get();
    match handle.lock() {
        Ok(shell) => render_input(shell.composer(), &status.get()),
        Err(_) => Text::new("inline shell lock is poisoned")
            .color(Color::Red)
            .into_element(),
    }
}

fn commit_submission(
    shell: &mut ExampleShell,
    text: String,
    width: u16,
    status: &Signal<String>,
    committed: &Signal<u64>,
) {
    let Some(token) = shell
        .composer()
        .pending_submission()
        .map(|pending| pending.token())
    else {
        status.set("submitted composer has no pending token".to_owned());
        return;
    };
    let next = match committed.get().checked_add(1) {
        Some(next) => next,
        None => {
            acknowledge_failure(shell, token, status, "message identity exhausted");
            return;
        }
    };
    let id = shell
        .live_messages()
        .last()
        .map_or(MessageId::new(next), |message| message.id());
    if shell.live_state(id) == Some(LiveState::AwaitingResolution) {
        acknowledge_failure(
            shell,
            token,
            status,
            "commit is latched; inspect the terminal and call InlineChatShell::resolve",
        );
        return;
    }
    if shell.live_state(id).is_none()
        && let Err(error) = shell.stream(id)
    {
        acknowledge_failure(shell, token, status, &format!("stream refused: {error}"));
        return;
    }
    let context = match ProjectionContext::new(width, ThemeIdentity::new(1)) {
        Ok(context) => context,
        Err(error) => {
            acknowledge_failure(
                shell,
                token,
                status,
                &format!("projection refused: {error}"),
            );
            return;
        }
    };
    let canonical = format!("You: {text}\nAssistant: Received message #{next}.");
    let report = match shell.finish(id, MessageRevision::INITIAL, &canonical, context) {
        Ok(report) => report,
        Err(error) => {
            acknowledge_failure(shell, token, status, &format!("commit refused: {error}"));
            return;
        }
    };
    match report {
        InlineCommitReport::Fixed {
            receipt,
            disposition,
        } => match shell.composer_mut().acknowledge_success(token) {
            Ok(()) => {
                committed.set(next);
                status.set(format!("fixed ({disposition:?}) as {receipt}"));
            }
            Err(error) => status.set(format!(
                "fixed, but composer acknowledgement failed: {error:?}"
            )),
        },
        InlineCommitReport::Retained { cause } => acknowledge_failure(
            shell,
            token,
            status,
            &format!("retained without terminal write: {cause}"),
        ),
        InlineCommitReport::Latched { evidence } => acknowledge_failure(
            shell,
            token,
            status,
            &format!("latched on undecidable terminal write: {evidence}"),
        ),
    }
}

fn acknowledge_failure(
    shell: &mut ExampleShell,
    token: rnk::components::chat::SubmissionToken,
    status: &Signal<String>,
    message: &str,
) {
    match shell.composer_mut().acknowledge_failure(token) {
        Ok(()) => status.set(message.to_owned()),
        Err(error) => status.set(format!("{message}; acknowledgement failed: {error:?}")),
    }
}

fn render_input(composer: &rnk::components::chat::ChatComposerState, status: &str) -> Element {
    let width = rnk::renderer::Terminal::size()
        .ok()
        .map(|(width, _)| width)
        .filter(|width| *width > 0);
    let Some(width) = width else {
        return Text::new("terminal size unavailable")
            .color(Color::Red)
            .into_element();
    };
    let projection = ComposerProjection::build(composer, width);
    Box::new()
        .flex_direction(FlexDirection::Column)
        .width(i32::from(width))
        .border_style(BorderStyle::Round)
        .border_color(Color::BrightCyan)
        .child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(
                    Text::new("❯ ")
                        .color(Color::BrightCyan)
                        .bold()
                        .into_element(),
                )
                .child(Text::new(projection.visible_slice().join("\n")).into_element())
                .child(Text::new("▏").color(Color::BrightCyan).into_element())
                .into_element(),
        )
        .child(Text::new(status).dim().into_element())
        .into_element()
}
