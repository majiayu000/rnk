//! What an inline chat leaves behind on a real terminal.
//!
//! Every other test in this area drives a `Write` in memory, which proves the
//! byte stream is right but says nothing about the terminal's own state. Raw
//! mode and cursor visibility are not bytes in a buffer — they are kernel
//! settings on a tty, and a program that fails to restore them leaves the user's
//! shell without echo, which is the single most user-hostile way a TUI can exit.
//!
//! So this drives the code under a real pty and reads that state directly.
//!
//! The pty master and its slave share one termios, so `tcgetattr` on the master
//! observes exactly what the child's terminal is set to — no self-reporting from
//! the child involved, which is what makes the assertion worth anything.
//!
//! Unix only: `termios` has no Windows equivalent to check.

#![cfg(unix)]

use std::io::{Read, Write};
use std::os::fd::RawFd;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use rnk::components::chat::scrollback::NativeTerminalSink;
use rnk::components::chat::{
    InlineChatShell, InlineCommitReport, MessageId, MessageRevision, ProjectionContext,
    ScrollbackNamespace, ThemeIdentity,
};
use rnk::renderer::Terminal;

/// Set on the child so the same test binary knows which side it is running as.
const CHILD_MARKER: &str = "RNK_PTY_CHILD";

/// Printed once the child holds raw mode, so the parent knows when to look.
///
/// The child then *blocks* until the parent replies. Printing alone is not
/// enough: seeing the marker in the output stream says only that the bytes
/// arrived, not that the child is still holding the terminal. On a fast machine
/// one `read` returns the whole session, restore included, and a termios sample
/// taken then measures the restored state and passes for the wrong reason.
const RAW_MODE_HELD: &str = "<<RAW-MODE-HELD>>";

/// Written by the parent to release the child once it has sampled the termios.
const RELEASE: u8 = b'\n';

/// Printed after the child has restored the terminal.
const RESTORED: &str = "<<RESTORED>>";

/// The two lines the child commits into the terminal's own scrollback.
const FIRST_LINE: &str = "committed-line-alpha";
const SECOND_LINE: &str = "committed-line-beta";

/// How many times the child announces the second message as finished.
const REPEATED_TERMINAL_EVENTS: usize = 3;

fn is_child() -> bool {
    std::env::var_os(CHILD_MARKER).is_some()
}

/// The child: enters inline mode, commits, and restores the terminal.
///
/// Runs as a `#[test]` because that is the only entry point an integration-test
/// binary exposes; the parent invokes it by name. It no-ops in the parent.
#[test]
fn pty_child_entrypoint() {
    if !is_child() {
        return;
    }

    let mut terminal = Terminal::new();
    terminal.enter_inline().expect("inline mode");
    print!("{RAW_MODE_HELD}\r\n");
    flush();
    // Hold the terminal until the parent has looked at it. This is the whole
    // handshake: without it the assertion depends on the parent winning a race
    // it has no way to win reliably.
    await_release();

    let mut shell = InlineChatShell::new(
        ScrollbackNamespace::new("pty-test").expect("non-empty"),
        NativeTerminalSink::new(std::io::stdout()),
    );
    let context = ProjectionContext::new(80, ThemeIdentity::new(1)).expect("non-zero width");

    commit(&mut shell, 1, FIRST_LINE, context, 1);
    commit(
        &mut shell,
        2,
        SECOND_LINE,
        context,
        REPEATED_TERMINAL_EVENTS,
    );

    terminal.exit_inline().expect("terminal restored");
    print!("{RESTORED}\r\n");
    flush();
}

fn commit<S: rnk::components::chat::ScrollbackSink>(
    shell: &mut InlineChatShell<S>,
    id: u64,
    text: &str,
    context: ProjectionContext,
    terminal_events: usize,
) {
    for _ in 0..terminal_events {
        let report = shell
            .finish(MessageId::new(id), MessageRevision::INITIAL, text, context)
            .expect("not latched");
        assert!(
            matches!(report, InlineCommitReport::Fixed { .. }),
            "the pty accepted the write, so the commit must be confirmed"
        );
    }
}

fn flush() {
    use std::io::Write;
    std::io::stdout().flush().expect("flush");
}

/// Blocks until the parent writes one byte to the pty.
fn await_release() {
    let mut byte = [0u8; 1];
    let mut stdin = std::io::stdin();
    loop {
        match stdin.read(&mut byte) {
            Ok(0) => panic!("the parent closed the pty without releasing the child"),
            Ok(_) => return,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => panic!("reading the release byte failed: {error}"),
        }
    }
}

/// Reads the pty's termios, which is shared between master and slave.
fn termios_of(fd: RawFd) -> libc::termios {
    let mut termios = unsafe { std::mem::zeroed::<libc::termios>() };
    // SAFETY: `fd` is an open pty master owned by the caller, and `termios` is a
    // correctly sized, writable `libc::termios`.
    let result = unsafe { libc::tcgetattr(fd, &mut termios) };
    assert_eq!(result, 0, "tcgetattr on the pty master must succeed");
    termios
}

fn echo_enabled(termios: &libc::termios) -> bool {
    termios.c_lflag & libc::ECHO != 0
}

fn canonical_mode(termios: &libc::termios) -> bool {
    termios.c_lflag & libc::ICANON != 0
}

#[test]
fn an_inline_chat_commits_into_scrollback_and_restores_the_terminal() {
    if is_child() {
        return;
    }

    let pty = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("a pty pair");
    let master_fd = pty
        .master
        .as_raw_fd()
        .expect("a unix pty master exposes its descriptor");

    let before = termios_of(master_fd);
    assert!(
        echo_enabled(&before) && canonical_mode(&before),
        "a fresh pty starts in cooked mode; the restore assertion is meaningless otherwise"
    );

    let mut command = CommandBuilder::new(
        std::env::current_exe().expect("the running test binary is addressable"),
    );
    command.arg("--exact");
    command.arg("pty_child_entrypoint");
    command.arg("--nocapture");
    command.env(CHILD_MARKER, "1");
    // libtest colours its own output; plain text keeps the marker search honest.
    command.env("NO_COLOR", "1");

    let mut child = pty.slave.spawn_command(command).expect("spawn the child");
    let mut reader = pty.master.try_clone_reader().expect("a reader");
    let mut writer = pty.master.take_writer().expect("a writer");
    drop(pty.slave);

    let mut output = String::new();
    let mut raw_mode_while_held = None;
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut buffer = [0u8; 4096];

    loop {
        if Instant::now() > deadline {
            panic!("the child never restored the terminal; saw:\n{output}");
        }
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => output.push_str(&String::from_utf8_lossy(&buffer[..count])),
            Err(error) => panic!("reading the pty failed: {error}\nsaw:\n{output}"),
        }
        // Sampled while the child is blocked waiting to be released, so it
        // provably still holds the terminal. Sampling on the marker alone would
        // be a race: one `read` can return the whole session, restore included,
        // and the sample would measure the restored state and pass for the wrong
        // reason.
        if raw_mode_while_held.is_none() && output.contains(RAW_MODE_HELD) {
            raw_mode_while_held = Some(termios_of(master_fd));
            writer.write_all(&[RELEASE]).expect("release the child");
            writer.flush().expect("flush the release byte");
        }
        if output.contains(RESTORED) {
            break;
        }
    }

    let held = raw_mode_while_held.expect("the child reported holding raw mode");
    assert!(
        !echo_enabled(&held),
        "inline mode must disable echo while it owns the terminal"
    );
    assert!(
        !canonical_mode(&held),
        "inline mode must leave canonical mode while it owns the terminal"
    );

    child.wait().expect("the child exits");

    let after = termios_of(master_fd);
    assert!(
        echo_enabled(&after),
        "echo must be restored on exit, or the user's shell is left silent"
    );
    assert!(
        canonical_mode(&after),
        "canonical mode must be restored on exit"
    );
    assert_eq!(
        after.c_lflag, before.c_lflag,
        "the terminal's line flags must be exactly what they were before"
    );

    // The cursor is hidden for the live region and shown again on exit. Order
    // matters: a show that precedes the hide would leave it hidden.
    let hide = output.find("\u{1b}[?25l").expect("the cursor was hidden");
    let show = output
        .rfind("\u{1b}[?25h")
        .expect("the cursor was shown again");
    assert!(
        hide < show,
        "the cursor must be restored after it was hidden, not before"
    );

    // The committed lines are in the pty's scrollback stream, once each — the
    // second despite three terminal events announcing it finished.
    assert_eq!(
        output.matches(FIRST_LINE).count(),
        1,
        "the first message must appear exactly once; saw:\n{output}"
    );
    assert_eq!(
        output.matches(SECOND_LINE).count(),
        1,
        "{REPEATED_TERMINAL_EVENTS} terminal events must still commit one line; saw:\n{output}"
    );
}
