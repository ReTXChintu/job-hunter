//! Job Hunter relay: account and device identity, online presence, and a
//! dumb message bus between one desktop and any number of paired phones.
//!
//! What this crate deliberately does **not** do: run any AI, talk to
//! Claude or Chrome, hold a MongoDB connection, or persist job/application
//! content. Its entire data footprint is accounts, devices, refresh
//! tokens and pairing codes in one local SQLite file. See
//! `docs/mobile-protocol.md` for the wire format.

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod extractor;
pub mod hub;
pub mod models;
pub mod rate_limit;
pub mod routes;
pub mod state;
pub mod util;

pub use config::RelayConfig;
pub use db::Db;
pub use routes::build_router;
pub use state::AppState;
