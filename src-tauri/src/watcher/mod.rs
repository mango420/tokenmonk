//! Log discovery, file watching and the ingest worker.
//!
//! * [`discover`]: finds `*.jsonl` files below `~/.claude/projects` (read only).
//! * [`ingest_file`]: incremental read of one file into the database.
//! * [`spawn_ingest`]: the single worker thread that owns the write
//!   connection, reacts to file events, limit polls and a minute tick, and
//!   hands every new `DashboardSnapshot` to an [`IngestPublisher`].
//!
//! The app layer and the game module plug in by implementing
//! [`IngestPublisher`]; nothing in this module needs to change for that.
//! Behaviour: docs/ARCHITECTURE.md, "Ingest" and "Error and empty states".

mod discover;
mod ingest;
mod publisher;
mod worker;

pub use discover::{discover, projects_dir, DiscoverError};
pub use ingest::{ingest_file, FileOutcome};
pub use publisher::{IngestPublisher, SnapshotCause};
pub use worker::{spawn_ingest, ClockZone, IngestConfig, IngestHandle};
