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
    UnknownResolution,
};
use rnk::prelude::*;

const MAX_VISIBLE_INPUT_LINES: NonZeroUsize = NonZeroUsize::new(4).expect("four is non-zero");
type ExampleShell = InlineChatShell<NativeTerminalSink<io::Stdout>>;

#[derive(Clone)]
pub(crate) struct PendingPublication {
    id: MessageId,
    text: String,
    width: u16,
    token: rnk::components::chat::SubmissionToken,
}

impl PendingPublication {
    pub(crate) fn new(
        id: MessageId,
        text: String,
        width: u16,
        token: rnk::components::chat::SubmissionToken,
    ) -> Self {
        Self {
            id,
            text,
            width,
            token,
        }
    }
}

pub(crate) enum HumanResolutionReport {
    AlreadyVisible,
    Retried(InlineCommitReport),
}

fn main() -> io::Result<()> {
    let _paste = BracketedPasteGuard::new()?;
    render(app).run()
}

pub(crate) fn app() -> Element {
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
    let pending_publication = use_signal(|| None::<PendingPublication>);

    let input_shell = shell.clone();
    let input_status = status.clone();
    let input_committed = committed.clone();
    let input_pending = pending_publication.clone();
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
        if key.ctrl && matches!(input, "r" | "v") {
            let Some(pending) = input_pending.get() else {
                input_status.set("no retained or latched publication".to_owned());
                return;
            };
            let resolution = if input == "v" {
                UnknownResolution::AlreadyVisible
            } else {
                UnknownResolution::NotVisible
            };
            match resolve_publication(&mut shell, &pending, resolution) {
                Ok(HumanResolutionReport::AlreadyVisible) => {
                    match shell.composer_mut().acknowledge_success(pending.token) {
                        Ok(()) => {
                            input_pending.set(None);
                            input_committed.set(pending.id.get());
                            input_status
                                .set("human confirmed the existing terminal line".to_owned());
                        }
                        Err(error) => input_status.set(format!(
                            "human resolution acknowledgement failed: {error:?}"
                        )),
                    }
                }
                Ok(HumanResolutionReport::Retried(InlineCommitReport::Fixed { .. })) => {
                    match shell.composer_mut().acknowledge_success(pending.token) {
                        Ok(()) => {
                            input_pending.set(None);
                            input_committed.set(pending.id.get());
                            input_status.set("human-approved retry was fixed".to_owned());
                        }
                        Err(error) => {
                            input_status.set(format!("retry acknowledgement failed: {error:?}"))
                        }
                    }
                }
                Ok(HumanResolutionReport::Retried(InlineCommitReport::Retained { cause })) => {
                    input_status.set(format!("human-approved retry retained: {cause}"));
                }
                Ok(HumanResolutionReport::Retried(InlineCommitReport::Latched { evidence })) => {
                    input_status.set(format!("human-approved retry is still latched: {evidence}"));
                }
                Err(error) => input_status.set(format!("human resolution refused: {error}")),
            }
            return;
        }
        match shell.handle_key(&ChatComposerKeyMap::new(), input, key) {
            InlineKeyOutcome::Submitted(text) => commit_submission(
                &mut shell,
                text,
                width,
                &input_status,
                &input_committed,
                &input_pending,
            ),
            InlineKeyOutcome::Cancelled => input_status.set("input cancelled".to_owned()),
            InlineKeyOutcome::Changed(_) => input_status.set("editing".to_owned()),
            InlineKeyOutcome::Handled => input_status.set("input handled".to_owned()),
            InlineKeyOutcome::Ignored => input_status.set("key ignored".to_owned()),
            InlineKeyOutcome::NotFocused => input_status.set("composer is not focused".to_owned()),
        }
    });

    let paste_shell = shell.clone();
    let paste_status = status.clone();
    use_paste(move |event| {
        let handle = paste_shell.get();
        let mut shell = match handle.lock() {
            Ok(shell) => shell,
            Err(_) => {
                paste_status.set("inline shell lock is poisoned during paste".to_owned());
                return;
            }
        };
        match shell.handle_key(&ChatComposerKeyMap::new(), event.content(), &Key::default()) {
            InlineKeyOutcome::Changed(_) => paste_status.set("pasted".to_owned()),
            InlineKeyOutcome::Handled => paste_status.set("paste handled".to_owned()),
            InlineKeyOutcome::Ignored => paste_status.set("paste ignored".to_owned()),
            InlineKeyOutcome::NotFocused => paste_status.set("composer is not focused".to_owned()),
            InlineKeyOutcome::Submitted(_) | InlineKeyOutcome::Cancelled => {
                paste_status.set("paste produced an invalid composer outcome".to_owned())
            }
        }
        drop(shell);
        paste_shell.set(handle);
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
    pending_publication: &Signal<Option<PendingPublication>>,
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
                pending_publication.set(None);
                committed.set(next);
                status.set(format!("fixed ({disposition:?}) as {receipt}"));
            }
            Err(error) => status.set(format!(
                "fixed, but composer acknowledgement failed: {error:?}"
            )),
        },
        InlineCommitReport::Retained { cause } => {
            pending_publication.set(Some(PendingPublication::new(id, canonical, width, token)));
            status.set(format!(
                "retained without terminal write: {cause}; Ctrl+R explicitly retries"
            ));
        }
        InlineCommitReport::Latched { evidence } => {
            pending_publication.set(Some(PendingPublication::new(id, canonical, width, token)));
            status.set(format!(
                "latched on undecidable terminal write: {evidence}; Ctrl+V confirms visible, Ctrl+R confirms absent"
            ));
        }
    }
}

pub(crate) fn resolve_publication<W: io::Write>(
    shell: &mut InlineChatShell<NativeTerminalSink<W>>,
    pending: &PendingPublication,
    resolution: UnknownResolution,
) -> Result<HumanResolutionReport, String> {
    match shell.live_state(pending.id) {
        Some(LiveState::AwaitingResolution) => {
            shell
                .resolve(pending.id, resolution)
                .map_err(|error| error.to_string())?;
            if resolution == UnknownResolution::AlreadyVisible {
                return Ok(HumanResolutionReport::AlreadyVisible);
            }
        }
        Some(LiveState::AwaitingRetry) if resolution == UnknownResolution::NotVisible => {}
        _ => return Err("publication is not eligible for this human resolution".to_owned()),
    }
    let context = ProjectionContext::new(pending.width, ThemeIdentity::new(1))
        .map_err(|error| error.to_string())?;
    shell
        .finish(pending.id, MessageRevision::INITIAL, &pending.text, context)
        .map(HumanResolutionReport::Retried)
        .map_err(|error| error.to_string())
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
