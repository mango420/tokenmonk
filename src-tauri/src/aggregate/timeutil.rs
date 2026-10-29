//! Local calendar helpers. Storage is UTC epoch ms; day boundaries are local.

use chrono::{DateTime, Duration, NaiveDate, TimeZone};

use crate::contracts::EpochMs;

/// Local calendar date of an instant.
pub fn local_date<Tz: TimeZone>(tz: &Tz, ms: EpochMs) -> NaiveDate {
    DateTime::from_timestamp_millis(ms)
        .unwrap_or_default()
        .with_timezone(tz)
        .date_naive()
}

/// Epoch ms of local midnight at the start of `date`. If midnight does not
/// exist (a DST gap), the first valid local time of that day is used.
pub fn day_start<Tz: TimeZone>(tz: &Tz, date: NaiveDate) -> EpochMs {
    for hour in 0..=3 {
        let Some(naive) = date.and_hms_opt(hour, 0, 0) else {
            continue;
        };
        if let Some(dt) = tz.from_local_datetime(&naive).earliest() {
            return dt.timestamp_millis();
        }
    }
    date.and_hms_opt(0, 0, 0)
        .map_or(0, |n| n.and_utc().timestamp_millis())
}

/// `date` shifted by whole days (saturating at the calendar limits).
pub fn add_days(date: NaiveDate, days: i64) -> NaiveDate {
    date.checked_add_signed(Duration::days(days))
        .unwrap_or(date)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    #[test]
    fn day_boundaries_follow_the_offset() {
        let tz = FixedOffset::east_opt(2 * 3600).unwrap();
        // 2026-10-28T23:30:00Z is 2026-10-29 01:30 local.
        let ms = 1_793_230_200_000;
        let date = local_date(&tz, ms);
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 10, 29).unwrap());
        // Local midnight 2026-10-29 is 2026-10-28T22:00:00Z.
        assert_eq!(day_start(&tz, date), 1_793_224_800_000);
        assert_eq!(
            add_days(date, -7),
            NaiveDate::from_ymd_opt(2026, 10, 22).unwrap()
        );
    }
}
