//! Season helper (docs/ARCHITECTURE.md, "Season and easter egg").
//!
//! The season is computed from local time. The tray icon needs it, so it
//! lives here as a pure function plus a thin wrapper around the clock.

use chrono::{Datelike, Local};

use crate::contracts::app::{Season, HALLOWEEN_END, HALLOWEEN_START};

/// Season for a local calendar date; both Halloween bounds are inclusive.
pub fn season_for(month: u32, day: u32) -> Season {
    if (month, day) >= HALLOWEEN_START && (month, day) <= HALLOWEEN_END {
        Season::Halloween
    } else {
        Season::Regular
    }
}

/// Season right now, in local time.
pub fn current_season() -> Season {
    let now = Local::now();
    season_for(now.month(), now.day())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halloween_bounds_are_inclusive() {
        assert_eq!(season_for(10, 23), Season::Regular);
        assert_eq!(season_for(10, 24), Season::Halloween);
        assert_eq!(season_for(10, 31), Season::Halloween);
        assert_eq!(season_for(11, 1), Season::Halloween);
        assert_eq!(season_for(11, 2), Season::Halloween);
        assert_eq!(season_for(11, 3), Season::Regular);
    }

    #[test]
    fn rest_of_the_year_is_regular() {
        assert_eq!(season_for(1, 1), Season::Regular);
        assert_eq!(season_for(7, 15), Season::Regular);
        assert_eq!(season_for(12, 24), Season::Regular);
    }
}
