//! Subscription limits: [`LimitSource`] implementations and the polling worker.
//!
//! v0.1 reads the file written by the status-line bridge
//! ([`StatuslineSource`]); the bridge itself is [`bridge`]. No network, no
//! credentials (docs/ARCHITECTURE.md, "Limits"). Another source is one more
//! [`LimitSource`] implementation handed to [`spawn`]; nothing else changes.
//! [`UnavailableSource`] stays for tests.

pub mod bridge;
pub mod statusline;
pub mod unavailable;
pub mod worker;

pub use statusline::StatuslineSource;
pub use unavailable::UnavailableSource;
pub use worker::{spawn, spawn_with, LimitPoll, LimitsHandle, WorkerConfig};

use std::sync::Arc;

use crate::contracts::limits::LimitSource;

/// The source this build uses.
pub fn default_source() -> Arc<dyn LimitSource> {
    Arc::new(StatuslineSource::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_source_is_the_statusline_file() {
        assert_eq!(default_source().id(), "statusline");
    }
}
