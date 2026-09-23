//! Local-first persistence.
//!
//! All reads and writes go to `LocalStore` (JSON documents on disk, cached in
//! memory). Every write is also queued for MongoDB Atlas; `sync::SyncWorker`
//! drains the queue whenever Atlas is reachable and pulls remote changes on
//! start. The app therefore keeps working when MongoDB is unavailable and
//! nothing is silently lost.

pub mod local;
pub mod mongo;
pub mod sync;

pub use local::{LocalStore, PendingOp, SyncOp};
pub use mongo::MongoClientHandle;
pub use sync::{SyncStatus, SyncWorker};
