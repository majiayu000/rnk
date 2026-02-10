//! Unified command type for side effects and typed messages.
//!
//! `Cmd<M>` is the single command representation in rnk. Use `Cmd<()>`
//! for side-effect-only flows, and `Cmd<MyMsg>` when you want commands to
//! produce typed messages.

use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

use super::{ExecConfig, ExecResult};

/// Application-level messages handled by the framework.
#[derive(Debug, Clone, Default)]
pub enum AppMsg {
    /// Window/terminal resize event.
    WindowResize { width: u16, height: u16 },
    /// Keyboard input (raw key string).
    KeyInput(String),
    /// Timer tick with timestamp.
    Tick(Instant),
    /// Focus changed to a new element.
    FocusChanged(Option<String>),
    /// Blur event (element lost focus).
    Blur,
    /// No-op message.
    #[default]
    None,
}

/// A boxed message that can hold any type.
pub struct BoxedMsg(Box<dyn Any + Send + 'static>);

impl BoxedMsg {
    /// Create a new boxed message.
    pub fn new<M: Any + Send + 'static>(msg: M) -> Self {
        BoxedMsg(Box::new(msg))
    }

    /// Try to downcast to a specific message type.
    pub fn downcast<M: Any + 'static>(self) -> Result<M, Self> {
        match self.0.downcast::<M>() {
            Ok(msg) => Ok(*msg),
            Err(boxed) => Err(BoxedMsg(boxed)),
        }
    }

    /// Try to get a reference to the inner message.
    pub fn downcast_ref<M: Any + 'static>(&self) -> Option<&M> {
        self.0.downcast_ref::<M>()
    }

    /// Check if this message is of a specific type.
    pub fn is<M: Any + 'static>(&self) -> bool {
        self.0.is::<M>()
    }
}

impl std::fmt::Debug for BoxedMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BoxedMsg(...)")
    }
}

/// Terminal control commands.
///
/// These are simple, stateless commands that control terminal behavior.
/// They don't carry message types and are handled directly by the renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalCmd {
    /// Clear the terminal screen.
    ClearScreen,
    /// Hide the terminal cursor.
    HideCursor,
    /// Show the terminal cursor.
    ShowCursor,
    /// Set the terminal window title.
    SetWindowTitle(String),
    /// Request the current window size.
    WindowSize,
    /// Enter alternate screen buffer.
    EnterAltScreen,
    /// Exit alternate screen buffer.
    ExitAltScreen,
    /// Enable mouse support.
    EnableMouse,
    /// Disable mouse support.
    DisableMouse,
    /// Enable bracketed paste mode.
    EnableBracketedPaste,
    /// Disable bracketed paste mode.
    DisableBracketedPaste,
}

/// Unified command type.
///
/// - `Cmd<()>`: side-effect commands used by runtime/hooks.
/// - `Cmd<M>`: typed commands that produce messages of type `M`.
#[derive(Default)]
pub enum Cmd<M = ()>
where
    M: Send + 'static,
{
    /// No-op command.
    #[default]
    None,

    /// Execute multiple commands concurrently.
    Batch(Vec<Cmd<M>>),

    /// Execute multiple commands sequentially.
    Sequence(Vec<Cmd<M>>),

    /// Execute an async task that produces a message.
    Perform {
        future: Pin<Box<dyn Future<Output = M> + Send + 'static>>,
    },

    /// Sleep for a duration, then execute another command.
    Sleep {
        duration: Duration,
        then: Box<Cmd<M>>,
    },

    /// Produce a message after waiting for a duration.
    Tick {
        duration: Duration,
        msg_fn: Box<dyn FnOnce(Instant) -> M + Send + 'static>,
    },

    /// Produce a message aligned to system clock boundaries.
    Every {
        duration: Duration,
        msg_fn: Box<dyn FnOnce(Instant) -> M + Send + 'static>,
    },

    /// Execute an external interactive process.
    Exec {
        config: ExecConfig,
        msg_fn: Box<dyn FnOnce(ExecResult) -> M + Send + 'static>,
    },

    /// Terminal control command.
    Terminal(TerminalCmd),
}

impl<M> Cmd<M>
where
    M: Send + 'static,
{
    /// Create a no-op command.
    pub fn none() -> Self {
        Cmd::None
    }

    /// Create a batch command that executes multiple commands concurrently.
    pub fn batch(cmds: impl IntoIterator<Item = Cmd<M>>) -> Self {
        let mut cmds: Vec<Cmd<M>> = cmds
            .into_iter()
            .filter(|cmd| !matches!(cmd, Cmd::None))
            .collect();

        match cmds.len() {
            0 => Cmd::None,
            1 => cmds.pop().unwrap(),
            _ => Cmd::Batch(cmds),
        }
    }

    /// Create a sequence command that executes multiple commands in order.
    pub fn sequence(cmds: impl IntoIterator<Item = Cmd<M>>) -> Self {
        let mut cmds: Vec<Cmd<M>> = cmds
            .into_iter()
            .filter(|cmd| !matches!(cmd, Cmd::None))
            .collect();

        match cmds.len() {
            0 => Cmd::None,
            1 => cmds.pop().unwrap(),
            _ => Cmd::Sequence(cmds),
        }
    }

    /// Create a command that executes an async function and produces a message.
    pub fn perform<F, Fut>(f: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = M> + Send + 'static,
    {
        Cmd::Perform {
            future: Box::pin(async move { f().await }),
        }
    }

    /// Create a command that sleeps for a duration.
    pub fn sleep(duration: Duration) -> Self {
        Cmd::Sleep {
            duration,
            then: Box::new(Cmd::None),
        }
    }

    /// Create a tick command that produces a message after a duration.
    pub fn tick<F>(duration: Duration, msg_fn: F) -> Self
    where
        F: FnOnce(Instant) -> M + Send + 'static,
    {
        Cmd::Tick {
            duration,
            msg_fn: Box::new(msg_fn),
        }
    }

    /// Create a command that ticks in sync with the system clock.
    pub fn every<F>(duration: Duration, msg_fn: F) -> Self
    where
        F: FnOnce(Instant) -> M + Send + 'static,
    {
        Cmd::Every {
            duration,
            msg_fn: Box::new(msg_fn),
        }
    }

    /// Execute an external interactive process.
    pub fn exec<F>(config: ExecConfig, msg_fn: F) -> Self
    where
        F: FnOnce(ExecResult) -> M + Send + 'static,
    {
        Cmd::Exec {
            config,
            msg_fn: Box::new(msg_fn),
        }
    }

    /// Execute an external command with simple arguments.
    pub fn exec_cmd<F>(program: &str, args: &[&str], msg_fn: F) -> Self
    where
        F: FnOnce(ExecResult) -> M + Send + 'static,
    {
        let config = ExecConfig::new(program).args(args.iter().map(|s| s.to_string()));
        Cmd::exec(config, msg_fn)
    }

    /// Clear the terminal screen.
    pub fn clear_screen() -> Self {
        Cmd::Terminal(TerminalCmd::ClearScreen)
    }

    /// Hide the terminal cursor.
    pub fn hide_cursor() -> Self {
        Cmd::Terminal(TerminalCmd::HideCursor)
    }

    /// Show the terminal cursor.
    pub fn show_cursor() -> Self {
        Cmd::Terminal(TerminalCmd::ShowCursor)
    }

    /// Set the terminal window title.
    pub fn set_window_title(title: impl Into<String>) -> Self {
        Cmd::Terminal(TerminalCmd::SetWindowTitle(title.into()))
    }

    /// Request the current window size.
    pub fn window_size() -> Self {
        Cmd::Terminal(TerminalCmd::WindowSize)
    }

    /// Enter alternate screen buffer.
    pub fn enter_alt_screen() -> Self {
        Cmd::Terminal(TerminalCmd::EnterAltScreen)
    }

    /// Exit alternate screen buffer.
    pub fn exit_alt_screen() -> Self {
        Cmd::Terminal(TerminalCmd::ExitAltScreen)
    }

    /// Enable mouse support.
    pub fn enable_mouse() -> Self {
        Cmd::Terminal(TerminalCmd::EnableMouse)
    }

    /// Disable mouse support.
    pub fn disable_mouse() -> Self {
        Cmd::Terminal(TerminalCmd::DisableMouse)
    }

    /// Enable bracketed paste mode.
    pub fn enable_bracketed_paste() -> Self {
        Cmd::Terminal(TerminalCmd::EnableBracketedPaste)
    }

    /// Disable bracketed paste mode.
    pub fn disable_bracketed_paste() -> Self {
        Cmd::Terminal(TerminalCmd::DisableBracketedPaste)
    }

    /// Chain this command with another command.
    pub fn and_then(self, next: Cmd<M>) -> Self {
        match self {
            Cmd::None => next,
            Cmd::Sleep { duration, then } => {
                let chained = then.and_then(next);
                Cmd::Sleep {
                    duration,
                    then: Box::new(chained),
                }
            }
            other => Cmd::batch(vec![other, next]),
        }
    }

    /// Check if this command is a no-op.
    pub fn is_none(&self) -> bool {
        matches!(self, Cmd::None)
    }

    /// Map command messages to a different type.
    pub fn map<N, F>(self, f: F) -> Cmd<N>
    where
        N: Send + 'static,
        F: FnOnce(M) -> N + Send + 'static + Clone,
    {
        match self {
            Cmd::None => Cmd::None,
            Cmd::Batch(cmds) => Cmd::Batch(cmds.into_iter().map(|c| c.map(f.clone())).collect()),
            Cmd::Sequence(cmds) => {
                Cmd::Sequence(cmds.into_iter().map(|c| c.map(f.clone())).collect())
            }
            Cmd::Perform { future } => Cmd::Perform {
                future: Box::pin(async move {
                    let msg = future.await;
                    f(msg)
                }),
            },
            Cmd::Sleep { duration, then } => Cmd::Sleep {
                duration,
                then: Box::new(then.map(f)),
            },
            Cmd::Tick { duration, msg_fn } => Cmd::Tick {
                duration,
                msg_fn: Box::new(move |t| f(msg_fn(t))),
            },
            Cmd::Every { duration, msg_fn } => Cmd::Every {
                duration,
                msg_fn: Box::new(move |t| f(msg_fn(t))),
            },
            Cmd::Exec { config, msg_fn } => Cmd::Exec {
                config,
                msg_fn: Box::new(move |r| f(msg_fn(r))),
            },
            Cmd::Terminal(tc) => Cmd::Terminal(tc),
        }
    }
}

impl<M> std::fmt::Debug for Cmd<M>
where
    M: Send + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::None => write!(f, "Cmd::None"),
            Cmd::Batch(cmds) => f.debug_tuple("Cmd::Batch").field(cmds).finish(),
            Cmd::Sequence(cmds) => f.debug_tuple("Cmd::Sequence").field(cmds).finish(),
            Cmd::Perform { .. } => write!(f, "Cmd::Perform {{ ... }}"),
            Cmd::Sleep { duration, then } => f
                .debug_struct("Cmd::Sleep")
                .field("duration", duration)
                .field("then", then)
                .finish(),
            Cmd::Tick { duration, .. } => f
                .debug_struct("Cmd::Tick")
                .field("duration", duration)
                .finish(),
            Cmd::Every { duration, .. } => f
                .debug_struct("Cmd::Every")
                .field("duration", duration)
                .finish(),
            Cmd::Exec { config, .. } => {
                f.debug_struct("Cmd::Exec").field("config", config).finish()
            }
            Cmd::Terminal(tc) => write!(f, "Cmd::Terminal({:?})", tc),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum TestMsg {
        Loaded(String),
        Tick(u64),
    }

    #[test]
    fn test_cmd_none_default() {
        let cmd: Cmd<TestMsg> = Cmd::none();
        assert!(cmd.is_none());

        let default_cmd: Cmd<TestMsg> = Cmd::default();
        assert!(default_cmd.is_none());
    }

    #[test]
    fn test_cmd_batch_and_sequence() {
        let batch: Cmd<TestMsg> = Cmd::batch(vec![
            Cmd::sleep(Duration::from_millis(10)),
            Cmd::sleep(Duration::from_millis(20)),
        ]);
        assert!(matches!(batch, Cmd::Batch(_)));

        let seq: Cmd<TestMsg> = Cmd::sequence(vec![
            Cmd::sleep(Duration::from_millis(10)),
            Cmd::sleep(Duration::from_millis(20)),
        ]);
        assert!(matches!(seq, Cmd::Sequence(_)));
    }

    #[test]
    fn test_cmd_perform_tick_every_exec() {
        let perform: Cmd<TestMsg> = Cmd::perform(|| async { TestMsg::Loaded("ok".into()) });
        assert!(matches!(perform, Cmd::Perform { .. }));

        let tick: Cmd<TestMsg> = Cmd::tick(Duration::from_secs(1), |_| TestMsg::Tick(1));
        assert!(matches!(tick, Cmd::Tick { .. }));

        let every: Cmd<TestMsg> = Cmd::every(Duration::from_secs(1), |_| TestMsg::Tick(2));
        assert!(matches!(every, Cmd::Every { .. }));

        let exec: Cmd<TestMsg> = Cmd::exec(ExecConfig::new("echo").arg("hi"), |_| {
            TestMsg::Loaded("done".into())
        });
        assert!(matches!(exec, Cmd::Exec { .. }));
    }

    #[test]
    fn test_cmd_and_then_chains_sleep() {
        let cmd: Cmd<TestMsg> = Cmd::sleep(Duration::from_secs(1))
            .and_then(Cmd::sleep(Duration::from_secs(2)))
            .and_then(Cmd::sleep(Duration::from_secs(3)));

        assert!(matches!(cmd, Cmd::Sleep { .. }));
    }

    #[test]
    fn test_cmd_map_message_type() {
        #[derive(Debug, PartialEq)]
        enum ParentMsg {
            Child(TestMsg),
        }

        let child_cmd: Cmd<TestMsg> = Cmd::batch(vec![
            Cmd::sleep(Duration::from_secs(1)),
            Cmd::perform(|| async { TestMsg::Loaded("data".into()) }),
        ]);

        let parent_cmd: Cmd<ParentMsg> = child_cmd.map(ParentMsg::Child);
        assert!(matches!(parent_cmd, Cmd::Batch(_)));
    }

    #[test]
    fn test_terminal_cmd_variants_exist() {
        assert!(matches!(
            Cmd::<()>::clear_screen(),
            Cmd::Terminal(TerminalCmd::ClearScreen)
        ));
        assert!(matches!(
            Cmd::<()>::hide_cursor(),
            Cmd::Terminal(TerminalCmd::HideCursor)
        ));
        assert!(matches!(
            Cmd::<()>::show_cursor(),
            Cmd::Terminal(TerminalCmd::ShowCursor)
        ));
        assert!(matches!(
            Cmd::<()>::window_size(),
            Cmd::Terminal(TerminalCmd::WindowSize)
        ));
        assert!(matches!(
            Cmd::<()>::enter_alt_screen(),
            Cmd::Terminal(TerminalCmd::EnterAltScreen)
        ));
        assert!(matches!(
            Cmd::<()>::exit_alt_screen(),
            Cmd::Terminal(TerminalCmd::ExitAltScreen)
        ));
        assert!(matches!(
            Cmd::<()>::enable_mouse(),
            Cmd::Terminal(TerminalCmd::EnableMouse)
        ));
        assert!(matches!(
            Cmd::<()>::disable_mouse(),
            Cmd::Terminal(TerminalCmd::DisableMouse)
        ));
        assert!(matches!(
            Cmd::<()>::enable_bracketed_paste(),
            Cmd::Terminal(TerminalCmd::EnableBracketedPaste)
        ));
        assert!(matches!(
            Cmd::<()>::disable_bracketed_paste(),
            Cmd::Terminal(TerminalCmd::DisableBracketedPaste)
        ));
    }

    #[test]
    fn test_app_msg_default() {
        assert!(matches!(AppMsg::default(), AppMsg::None));
    }

    #[test]
    fn test_boxed_msg_downcast() {
        let msg = BoxedMsg::new(TestMsg::Tick(42));
        assert!(msg.is::<TestMsg>());

        let downcasted = msg.downcast::<TestMsg>();
        assert!(downcasted.is_ok());
        assert_eq!(downcasted.unwrap(), TestMsg::Tick(42));
    }
}
