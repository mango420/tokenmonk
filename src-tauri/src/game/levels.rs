//! Level curve and titles. Pure functions of the XP total and the mode.

use super::constants::{LEVEL_BASE_XP, LEVEL_STEP_XP, TITLES_DIY, TITLES_MAX};
use crate::contracts::Mode;

/// Where an XP total sits on the level curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelInfo {
    pub level: u32,
    /// XP earned inside the level.
    pub into_level: u64,
    /// XP the level spans.
    pub for_level: u64,
}

impl LevelInfo {
    /// `0.0..=1.0`.
    pub fn progress(&self) -> f64 {
        if self.for_level == 0 {
            0.0
        } else {
            self.into_level as f64 / self.for_level as f64
        }
    }
}

/// XP spanned by `level` (levels start at 1).
pub fn xp_for_level(level: u32) -> u64 {
    LEVEL_BASE_XP + LEVEL_STEP_XP * u64::from(level)
}

/// Level, XP into it and XP it spans for an XP total.
pub fn level_info(xp_total: u64) -> LevelInfo {
    let mut level = 1u32;
    let mut rest = xp_total;
    loop {
        let span = xp_for_level(level);
        if rest < span {
            return LevelInfo {
                level,
                into_level: rest,
                for_level: span,
            };
        }
        rest -= span;
        level += 1;
    }
}

/// Title at `level`. Before onboarding (`NoneYet`) the Max titles apply, as
/// the popup falls back to Max it out.
pub fn title_for(mode: Mode, level: u32) -> &'static str {
    let table: &[(u32, &str)] = match mode {
        Mode::Diy => &TITLES_DIY,
        Mode::Max | Mode::NoneYet => &TITLES_MAX,
    };
    table
        .iter()
        .rev()
        .find(|(first, _)| level >= *first)
        .map(|(_, title)| *title)
        .unwrap_or(table[0].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_matches_the_design_spans() {
        assert_eq!(xp_for_level(4), 1_160);
        assert_eq!(xp_for_level(7), 1_850);
    }

    #[test]
    fn level_boundaries() {
        assert_eq!(level_info(0).level, 1);
        let l1 = xp_for_level(1);
        assert_eq!(level_info(l1 - 1).level, 1);
        let at = level_info(l1);
        assert_eq!((at.level, at.into_level), (2, 0));
        let sum3 = l1 + xp_for_level(2) + xp_for_level(3);
        let at = level_info(sum3 + 560);
        assert_eq!((at.level, at.into_level, at.for_level), (4, 560, 1_160));
        assert!((at.progress() - 560.0 / 1_160.0).abs() < 1e-12);
    }

    #[test]
    fn titles_follow_mode_and_level() {
        assert_eq!(title_for(Mode::Max, 7), "Token Squire");
        assert_eq!(title_for(Mode::Diy, 4), "Novice Monk");
        assert_eq!(title_for(Mode::Max, 9), "Rate-Limit Rambo");
        assert_eq!(title_for(Mode::Max, 8), "Token Squire");
        assert_eq!(title_for(Mode::Max, 99), "Token Titan");
        assert_eq!(title_for(Mode::NoneYet, 1), "Token Sprout");
    }
}
