//! Subscription limits: [`LimitSource`] implementations and the polling worker.
//!
//! v0.1 ships [`UnavailableSource`] only. No online source is implemented
//! because no endpoint could be verified from official documentation or
//! observed behaviour (see `docs/ARCHITECTURE.md`, "Limits"). A verified
//! source is added as one more [`LimitSource`] implementation next to it and
//! handed to [`spawn`]; nothing else changes.

pub mod unavailable;
pub mod worker;

pub use unavailable::UnavailableSource;
pub use worker::{spawn, spawn_with, LimitPoll, LimitsHandle, WorkerConfig};

use crate::contracts::limits::LimitSource;

/// The source this build uses.
pub fn default_source() -> Box<dyn LimitSource> {
    Box::new(UnavailableSource::no_source())
}
