//! Local-first persistence.
//!
//! All reads and writes go to `LocalStore` (JSON documents on disk, cached in
//! memory). Every write is also queued for the self-hosted backend (see
//! `apps/backend`, `docs/backend.md`); `sync::SyncWorker` drains the queue
//! whenever it is reachable and pulls remote changes on start. The app
//! therefore keeps working with no backend account signed in, and nothing is
//! silently lost when the backend is briefly unreachable.

pub mod backend;
pub mod local;
pub mod sync;

pub use backend::BackendClientHandle;
pub use local::{LocalStore, PendingOp, SyncOp};
pub use sync::{SyncStatus, SyncWorker};
