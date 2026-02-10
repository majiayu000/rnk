//! Command system for managing side effects.
//!
//! `Cmd<M>` is the unified command type:
//! - Use `Cmd<()>` for side-effect-only commands.
//! - Use `Cmd<MyMsg>` for type-safe message-producing commands.
//!
//! # Core command constructors
//!
//! - [`Cmd::none()`]
//! - [`Cmd::perform()`]
//! - [`Cmd::batch()`]
//! - [`Cmd::sequence()`]
//! - [`Cmd::sleep()`]
//! - [`Cmd::tick()`]
//! - [`Cmd::every()`]
//! - [`Cmd::exec()`]

mod core;
mod exec;
mod executor;
mod tasks;

pub use core::{AppMsg, BoxedMsg, Cmd, TerminalCmd};
pub use exec::{ExecConfig, ExecResult};
pub use executor::{CmdExecutor, RenderHandle, run_exec_process};
pub use tasks::{HttpRequest, HttpResponse, ProcessOutput};

pub(crate) use exec::ExecRequest;
