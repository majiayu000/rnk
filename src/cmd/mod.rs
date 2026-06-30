//! Command system for managing side effects.
//!
//! `Cmd<M>` is the unified command type:
//! - Use `Cmd<()>` for side-effect-only commands.
//! - Use `Cmd<MyMsg>` for type-safe message-producing commands.
//!
//! # Core command constructors
//!
//! - [`Cmd::none`](crate::cmd::Cmd::none)
//! - [`Cmd::perform`](crate::cmd::Cmd::perform)
//! - [`Cmd::batch`](crate::cmd::Cmd::batch)
//! - [`Cmd::sequence`](crate::cmd::Cmd::sequence)
//! - [`Cmd::sleep`](crate::cmd::Cmd::sleep)
//! - [`Cmd::tick`](crate::cmd::Cmd::tick)
//! - [`Cmd::every`](crate::cmd::Cmd::every)
//! - [`Cmd::exec`](crate::cmd::Cmd::exec)

mod core;
mod exec;
mod executor;
mod tasks;

pub use core::{AppMsg, BoxedMsg, Cmd, TerminalCmd};
pub use exec::{ExecConfig, ExecResult};
pub use executor::{CmdExecutor, CmdRenderNotifier, run_exec_process};
pub use tasks::{HttpRequest, HttpResponse, ProcessOutput};

pub(crate) use exec::ExecRequest;
