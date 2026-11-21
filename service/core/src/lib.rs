//! Core service: state machine, IPC envelope, settings and log redaction.
//!
//! Phase 2 in progress. Everything here is pure logic: no Windows API, no
//! network, no side effects. The contract in `contracts/ipc-v0.md` is the
//! source of truth; when this crate and the contract disagree, the contract
//! wins and the crate is wrong.

#![forbid(unsafe_code)]

pub mod ipc;
pub mod redact;
pub mod settings;
pub mod state;
pub mod url;
