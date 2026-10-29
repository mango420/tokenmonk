//! A source that never has data.

use crate::contracts::limits::{LimitError, LimitReading, LimitReason, LimitSource};

/// Always fails with a fixed reason, so the app runs on token data alone and
/// the UI explains why (docs/DESIGN.md, "Limits unavailable and stale").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnavailableSource {
    reason: LimitReason,
    detail: &'static str,
}

impl UnavailableSource {
    pub fn new(reason: LimitReason, detail: &'static str) -> Self {
        Self { reason, detail }
    }

    /// The v0.1 default: this build has no verified limit endpoint.
    pub fn no_source() -> Self {
        Self::new(
            LimitReason::NoSource,
            "no verified limit endpoint in this build",
        )
    }
}

impl LimitSource for UnavailableSource {
    fn id(&self) -> &'static str {
        "none"
    }

    fn fetch(&self) -> Result<LimitReading, LimitError> {
        Err(LimitError {
            reason: self.reason,
            detail: self.detail.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_source_reports_no_source() {
        let source = UnavailableSource::no_source();
        assert_eq!(source.id(), "none");
        let err = source.fetch().unwrap_err();
        assert_eq!(err.reason, LimitReason::NoSource);
        assert!(!err.detail.is_empty());
    }
}
