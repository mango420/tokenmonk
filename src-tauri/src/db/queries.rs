//! Read helpers for the aggregate step. All time arguments are Unix epoch
//! milliseconds (UTC); calendar logic stays with the caller, except
//! [`Db::tokens_by_local_day`], which buckets by the given time zone.

use std::collections::BTreeMap;

use chrono::{NaiveDate, TimeZone};
use rusqlite::OptionalExtension;

use super::{Db, DbResult};
use crate::contracts::EpochMs;

/// Summed token columns of a set of messages.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenTotals {
    pub messages: u64,
    pub input: u64,
    pub output: u64,
    pub cache_creation: u64,
    /// Stored and reported separately; not part of [`TokenTotals::headline`].
    pub cache_read: u64,
}

impl TokenTotals {
    /// Headline tokens = input + output + cache creation (cache reads excluded).
    pub fn headline(&self) -> u64 {
        self.input + self.output + self.cache_creation
    }

    fn add(&mut self, other: &TokenTotals) {
        self.messages += other.messages;
        self.input += other.input;
        self.output += other.output;
        self.cache_creation += other.cache_creation;
        self.cache_read += other.cache_read;
    }
}

/// Totals of one local calendar day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayTotals {
    pub day: NaiveDate,
    pub totals: TokenTotals,
}

/// Totals of one session with its first and last message time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    pub session_id: String,
    pub totals: TokenTotals,
    pub first_ts: EpochMs,
    pub last_ts: EpochMs,
}

/// The most recent assistant message overall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatestMessage {
    pub session_id: String,
    pub ts: EpochMs,
}

fn u(v: i64) -> u64 {
    u64::try_from(v).unwrap_or(0)
}

const SUMS: &str = "COUNT(*), COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0), \
    COALESCE(SUM(cache_creation_tokens), 0), COALESCE(SUM(cache_read_tokens), 0)";

fn totals_from_row(r: &rusqlite::Row<'_>, at: usize) -> rusqlite::Result<TokenTotals> {
    Ok(TokenTotals {
        messages: u(r.get(at)?),
        input: u(r.get(at + 1)?),
        output: u(r.get(at + 2)?),
        cache_creation: u(r.get(at + 3)?),
        cache_read: u(r.get(at + 4)?),
    })
}

impl Db {
    /// Totals of messages with `from_ms <= ts < to_ms` (for a local day:
    /// local midnight to the next local midnight, converted to epoch ms).
    pub fn tokens_between(&self, from_ms: EpochMs, to_ms: EpochMs) -> DbResult<TokenTotals> {
        let sql = format!("SELECT {SUMS} FROM messages WHERE ts >= ?1 AND ts < ?2");
        Ok(self
            .conn()
            .query_row(&sql, [from_ms, to_ms], |r| totals_from_row(r, 0))?)
    }

    /// Totals of the trailing window `now_ms - window_ms <= ts <= now_ms`
    /// (burn rate, 5-hour window start via `window_ms`).
    pub fn tokens_trailing(&self, now_ms: EpochMs, window_ms: i64) -> DbResult<TokenTotals> {
        let sql = format!("SELECT {SUMS} FROM messages WHERE ts >= ?1 AND ts <= ?2");
        Ok(self
            .conn()
            .query_row(&sql, [now_ms.saturating_sub(window_ms), now_ms], |r| {
                totals_from_row(r, 0)
            })?)
    }

    /// Totals per local calendar day of `tz` for `from_ms <= ts < to_ms`.
    /// Days without messages are omitted; result is ordered by day.
    pub fn tokens_by_local_day<Tz: TimeZone>(
        &self,
        tz: &Tz,
        from_ms: EpochMs,
        to_ms: EpochMs,
    ) -> DbResult<Vec<DayTotals>> {
        let mut stmt = self.conn().prepare_cached(
            "SELECT ts, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens \
             FROM messages WHERE ts >= ?1 AND ts < ?2",
        )?;
        let rows = stmt.query_map([from_ms, to_ms], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                TokenTotals {
                    messages: 1,
                    input: u(r.get(1)?),
                    output: u(r.get(2)?),
                    cache_creation: u(r.get(3)?),
                    cache_read: u(r.get(4)?),
                },
            ))
        })?;
        let mut days: BTreeMap<NaiveDate, TokenTotals> = BTreeMap::new();
        for row in rows {
            let (ts, t) = row?;
            let Some(dt) = tz.timestamp_millis_opt(ts).single() else {
                continue;
            };
            days.entry(dt.date_naive()).or_default().add(&t);
        }
        Ok(days
            .into_iter()
            .map(|(day, totals)| DayTotals { day, totals })
            .collect())
    }

    /// All deduplicated messages of a session (main and subagent lines), any age.
    pub fn session_summary(&self, session_id: &str) -> DbResult<Option<SessionSummary>> {
        let sql = format!("SELECT {SUMS}, MIN(ts), MAX(ts) FROM messages WHERE session_id = ?1");
        Ok(self.conn().query_row(&sql, [session_id], |r| {
            let totals = totals_from_row(r, 0)?;
            if totals.messages == 0 {
                return Ok(None);
            }
            Ok(Some(SessionSummary {
                session_id: session_id.to_owned(),
                totals,
                first_ts: r.get(5)?,
                last_ts: r.get(6)?,
            }))
        })?)
    }

    /// Session and time of the most recent message overall (current-session rule).
    pub fn latest_message(&self) -> DbResult<Option<LatestMessage>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT session_id, ts FROM messages ORDER BY ts DESC, message_id DESC LIMIT 1",
                [],
                |r| {
                    Ok(LatestMessage {
                        session_id: r.get(0)?,
                        ts: r.get(1)?,
                    })
                },
            )
            .optional()?)
    }

    /// Number of stored messages.
    pub fn message_count(&self) -> DbResult<u64> {
        Ok(u(self.conn().query_row(
            "SELECT COUNT(*) FROM messages",
            [],
            |r| r.get(0),
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::UsageRecord;
    use chrono::FixedOffset;

    const HOUR: i64 = 3_600_000;
    // 2026-10-28T00:00:00Z
    const DAY0: i64 = 1_793_145_600_000;

    fn rec(
        id: &str,
        session: &str,
        ts: i64,
        input: u64,
        out: u64,
        cc: u64,
        cr: u64,
    ) -> UsageRecord {
        UsageRecord {
            message_id: id.into(),
            request_id: None,
            session_id: session.into(),
            agent_id: None,
            is_sidechain: false,
            ts,
            model: "model-x".into(),
            project_path: "/work/demo-app".into(),
            input_tokens: input,
            output_tokens: out,
            cache_creation_tokens: cc,
            cache_read_tokens: cr,
        }
    }

    fn seeded() -> Db {
        let mut db = Db::open_in_memory().unwrap();
        db.upsert_messages(&[
            rec("a", "s1", DAY0 + HOUR, 1, 10, 100, 1000),
            rec("b", "s1", DAY0 + 23 * HOUR + 30 * 60_000, 2, 20, 200, 2000),
            rec("c", "s2", DAY0 + 25 * HOUR, 4, 40, 400, 4000),
        ])
        .unwrap();
        db
    }

    #[test]
    fn between_is_half_open_and_separates_cache_reads() {
        let db = seeded();
        let t = db.tokens_between(DAY0, DAY0 + 24 * HOUR).unwrap();
        assert_eq!(t.messages, 2);
        assert_eq!((t.input, t.output, t.cache_creation), (3, 30, 300));
        assert_eq!(t.cache_read, 3000);
        assert_eq!(t.headline(), 333);
        let edge = db.tokens_between(DAY0 + HOUR, DAY0 + HOUR + 1).unwrap();
        assert_eq!(edge.messages, 1);
        let none = db.tokens_between(DAY0 + 2 * HOUR, DAY0 + 3 * HOUR).unwrap();
        assert_eq!(none, TokenTotals::default());
    }

    #[test]
    fn trailing_window() {
        let db = seeded();
        let now = DAY0 + 25 * HOUR;
        let t = db.tokens_trailing(now, 90 * 60_000).unwrap();
        assert_eq!(t.messages, 2); // b (1.5 h before, inclusive) and c
        let t = db.tokens_trailing(now, 60_000).unwrap();
        assert_eq!(t.messages, 1);
        assert_eq!(t.headline(), 444);
    }

    #[test]
    fn local_day_buckets_follow_the_time_zone() {
        let db = seeded();
        let utc = FixedOffset::east_opt(0).unwrap();
        let days = db
            .tokens_by_local_day(&utc, DAY0, DAY0 + 48 * HOUR)
            .unwrap();
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].day, NaiveDate::from_ymd_opt(2026, 10, 28).unwrap());
        assert_eq!(days[0].totals.messages, 2);
        assert_eq!(days[1].totals.messages, 1);

        // UTC+2 moves message b (23:30 UTC) into the next local day.
        let plus2 = FixedOffset::east_opt(2 * 3600).unwrap();
        let days = db
            .tokens_by_local_day(&plus2, DAY0, DAY0 + 48 * HOUR)
            .unwrap();
        assert_eq!(days[0].totals.messages, 1);
        assert_eq!(days[1].totals.messages, 2);
        assert_eq!(days[1].totals.cache_read, 6000);
    }

    #[test]
    fn session_summary_and_latest_message() {
        let db = seeded();
        let s = db.session_summary("s1").unwrap().unwrap();
        assert_eq!(s.totals.messages, 2);
        assert_eq!(s.totals.headline(), 333);
        assert_eq!(s.first_ts, DAY0 + HOUR);
        assert_eq!(s.last_ts, DAY0 + 23 * HOUR + 30 * 60_000);
        assert_eq!(db.session_summary("nope").unwrap(), None);
        assert_eq!(
            db.latest_message().unwrap(),
            Some(LatestMessage {
                session_id: "s2".into(),
                ts: DAY0 + 25 * HOUR
            })
        );
        assert_eq!(db.message_count().unwrap(), 3);
    }

    #[test]
    fn empty_database() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.latest_message().unwrap(), None);
        assert_eq!(
            db.tokens_between(0, i64::MAX).unwrap(),
            TokenTotals::default()
        );
        assert!(db
            .tokens_by_local_day(&chrono::Utc, 0, i64::MAX)
            .unwrap()
            .is_empty());
    }
}
