//! The desktop's half of the mobile companion app connection: register or
//! sign in to a self-hosted `job-hunter-relay`, keep one WebSocket alive,
//! and answer requests from paired phones by reusing the exact same
//! `AppContext` / `orchestrator` calls the Tauri UI uses. See
//! `docs/mobile-protocol.md` for the wire format and
//! `crates/job-hunter-relay` for the server this talks to.

pub mod client;
pub mod dispatch;
pub mod protocol;

pub use client::{RemoteClient, RemoteDevice, RemoteStatus};
