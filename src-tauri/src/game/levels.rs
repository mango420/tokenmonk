//! Level curve and titles. Pure functions of the XP total and the mode.

use super::constants::{LEVEL_FIRST_SPAN_XP, LEVEL_FLAT_FROM, LEVEL_STEP_XP, TITLE_TIERS};
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
    let n = level.clamp(1, LEVEL_FLAT_FROM - 1);
    LEVEL_FIRST_SPAN_XP + LEVEL_STEP_XP * u64::from(n - 1)
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

/// Title of the mode at `level`: it changes at the tier edges in
/// `TITLE_TIERS`. Before onboarding (`NoneYet`) the Max title applies, as the
/// popup falls back to Max it out.
pub fn title_for(mode: Mode, level: u32) -> &'static str {
    let tier = TITLE_TIERS
        .iter()
        .rev()
        .find(|t| level >= t.min_level)
        .unwrap_or(&TITLE_TIERS[0]);
    match mode {
        Mode::Diy => tier.diy,
        Mode::Max | Mode::NoneYet => tier.max,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_matches_the_design_spans() {
        assert_eq!(xp_for_level(1), 200);
        assert_eq!(xp_for_level(2), 320);
        assert_eq!(xp_for_level(25), 3_080);
        assert_eq!(xp_for_level(26), 3_080);
        assert_eq!(xp_for_level(400), 3_080);
        let reach = |n: u32| (1..n).map(xp_for_level).sum::<u64>();
        assert_eq!(reach(2), 200);
        assert_eq!(reach(5), 1_520);
        assert_eq!(reach(10), 6_120);
        assert_eq!(reach(20), 24_320);
        assert_eq!(reach(30), 53_320);
    }

    #[test]
    fn level_boundaries() {
        assert_eq!(level_info(0).level, 1);
        let l1 = xp_for_level(1);
        assert_eq!(level_info(l1 - 1).level, 1);
        let at = level_info(l1);
        assert_eq!((at.level, at.into_level), (2, 0));
        let sum3 = l1 + xp_for_level(2) + xp_for_level(3);
        let at = level_info(sum3 + 300);
        assert_eq!((at.level, at.into_level, at.for_level), (4, 300, 560));
        assert!((at.progress() - 300.0 / 560.0).abs() < 1e-12);
    }

    #[test]
    fn titles_change_at_the_tier_edges() {
        let max = [
            (1, "Token Tourist"),
            (4, "Token Tourist"),
            (5, "Prompt Regular"),
            (9, "Prompt Regular"),
            (10, "Context Hoarder"),
            (14, "Context Hoarder"),
            (15, "Cache Whisperer"),
            (19, "Cache Whisperer"),
            (20, "Limit Chaser"),
            (29, "Limit Chaser"),
            (30, "Rate-Limit Royalty"),
            (99, "Rate-Limit Royalty"),
        ];
        for (level, title) in max {
            assert_eq!(title_for(Mode::Max, level), title);
            assert_eq!(title_for(Mode::NoneYet, level), title);
        }
        let diy = [
            (4, "Budget Curious"),
            (5, "Quiet Spender"),
            (10, "Frugal Prompter"),
            (15, "Token Ascetic"),
            (20, "Zen Accountant"),
            (30, "Abbot of Restraint"),
        ];
        for (level, title) in diy {
            assert_eq!(title_for(Mode::Diy, level), title);
        }
    }
}
