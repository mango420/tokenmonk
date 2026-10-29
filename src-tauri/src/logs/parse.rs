//! Line parser for Claude Code JSONL logs (docs/LOG-FORMAT.md).

use chrono::DateTime;
use serde::{Deserialize, Deserializer};

use crate::contracts::EpochMs;

/// One assistant line reduced to the fields Tokenmonk stores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageRecord {
    pub message_id: String,
    pub request_id: Option<String>,
    pub session_id: String,
    pub agent_id: Option<String>,
    pub is_sidechain: bool,
    pub ts: EpochMs,
    pub model: String,
    /// `cwd` of the line; empty if the line has none.
    pub project_path: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
}

/// Why a line produced no record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// Empty or whitespace-only line.
    Blank,
    /// Not valid JSON, or not a JSON object of the expected shape.
    Malformed,
    /// A line type other than `assistant` (never carries tokens).
    NotAssistant,
    /// Client-made line (`<synthetic>` model or `isApiErrorMessage`).
    Synthetic,
    /// No `message.usage`.
    MissingUsage,
    /// A required field (`message.id`, `sessionId`, `message.model`,
    /// `timestamp`) is absent or unusable.
    MissingField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseOutcome {
    Record(UsageRecord),
    Skip(SkipReason),
}

#[derive(Deserialize)]
struct TypeProbe {
    #[serde(rename = "type", default, deserialize_with = "string_or_none")]
    kind: Option<String>,
}

/// A `type` that is not a string counts as absent.
fn string_or_none<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Ok(match serde_json::Value::deserialize(d)? {
        serde_json::Value::String(s) => Some(s),
        _ => None,
    })
}

#[derive(Deserialize)]
struct RawLine {
    timestamp: Option<String>,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    #[serde(rename = "isSidechain", default)]
    is_sidechain: Option<bool>,
    #[serde(rename = "agentId")]
    agent_id: Option<String>,
    cwd: Option<String>,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
    #[serde(rename = "isApiErrorMessage", default)]
    is_api_error: Option<bool>,
    message: Option<RawMessage>,
}

#[derive(Deserialize)]
struct RawMessage {
    id: Option<String>,
    model: Option<String>,
    usage: Option<RawUsage>,
}

#[derive(Deserialize)]
struct RawUsage {
    #[serde(default, deserialize_with = "lenient_u64")]
    input_tokens: u64,
    #[serde(default, deserialize_with = "lenient_u64")]
    output_tokens: u64,
    #[serde(default, deserialize_with = "lenient_u64")]
    cache_creation_input_tokens: u64,
    #[serde(default, deserialize_with = "lenient_u64")]
    cache_read_input_tokens: u64,
}

/// Token counts that are not non-negative integers count as 0.
fn lenient_u64<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(v.as_u64().unwrap_or(0))
}

/// Parses one log line (without the trailing newline).
pub fn parse_line(line: &str) -> ParseOutcome {
    use ParseOutcome::Skip;
    let line = line.trim();
    if line.is_empty() {
        return Skip(SkipReason::Blank);
    }
    // Decide on `type` first: other line types may have any shape elsewhere.
    match serde_json::from_str::<TypeProbe>(line) {
        Ok(TypeProbe { kind: Some(k) }) if k == "assistant" => {}
        Ok(_) => return Skip(SkipReason::NotAssistant),
        Err(_) => return Skip(SkipReason::Malformed),
    }
    let raw: RawLine = match serde_json::from_str(line) {
        Ok(raw) => raw,
        Err(_) => return Skip(SkipReason::Malformed),
    };
    if raw.is_api_error == Some(true) {
        return Skip(SkipReason::Synthetic);
    }
    let Some(message) = raw.message else {
        return Skip(SkipReason::MissingUsage);
    };
    if message.model.as_deref() == Some("<synthetic>") {
        return Skip(SkipReason::Synthetic);
    }
    let Some(usage) = message.usage else {
        return Skip(SkipReason::MissingUsage);
    };
    let (Some(message_id), Some(session_id), Some(model), Some(timestamp)) = (
        message.id.filter(|s| !s.is_empty()),
        raw.session_id.filter(|s| !s.is_empty()),
        message.model.filter(|s| !s.is_empty()),
        raw.timestamp,
    ) else {
        return Skip(SkipReason::MissingField);
    };
    let Ok(ts) = DateTime::parse_from_rfc3339(&timestamp).map(|t| t.timestamp_millis()) else {
        return Skip(SkipReason::MissingField);
    };
    ParseOutcome::Record(UsageRecord {
        message_id,
        request_id: raw.request_id,
        session_id,
        agent_id: raw.agent_id,
        is_sidechain: raw.is_sidechain.unwrap_or(false),
        ts,
        model,
        project_path: raw.cwd.unwrap_or_default(),
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cache_creation_tokens: usage.cache_creation_input_tokens,
        cache_read_tokens: usage.cache_read_input_tokens,
    })
}

/// Number of bytes up to and including the last `\n`; the rest is an
/// incomplete line to retry on the next read.
pub fn complete_prefix_len(buf: &[u8]) -> usize {
    buf.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1)
}

/// Skipped lines by reason (blank lines are not counted).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkipCounts {
    pub malformed: u64,
    pub not_assistant: u64,
    pub synthetic: u64,
    pub missing_usage: u64,
    pub missing_field: u64,
}

impl SkipCounts {
    pub fn total(&self) -> u64 {
        self.malformed
            + self.not_assistant
            + self.synthetic
            + self.missing_usage
            + self.missing_field
    }
}

/// Result of parsing a chunk of file bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChunkResult {
    pub records: Vec<UsageRecord>,
    pub skipped: SkipCounts,
    /// Bytes consumed (up to the last newline); advance the file offset by this.
    pub consumed: usize,
}

/// Parses all complete lines of `buf`; a trailing partial line is left
/// unconsumed. Invalid UTF-8 is replaced, which at worst makes the line
/// malformed.
pub fn parse_chunk(buf: &[u8]) -> ChunkResult {
    let consumed = complete_prefix_len(buf);
    let mut out = ChunkResult {
        consumed,
        ..ChunkResult::default()
    };
    for raw in buf[..consumed].split(|&b| b == b'\n') {
        let line = String::from_utf8_lossy(raw);
        match parse_line(&line) {
            ParseOutcome::Record(r) => out.records.push(r),
            ParseOutcome::Skip(SkipReason::Blank) => {}
            ParseOutcome::Skip(SkipReason::Malformed) => out.skipped.malformed += 1,
            ParseOutcome::Skip(SkipReason::NotAssistant) => out.skipped.not_assistant += 1,
            ParseOutcome::Skip(SkipReason::Synthetic) => out.skipped.synthetic += 1,
            ParseOutcome::Skip(SkipReason::MissingUsage) => out.skipped.missing_usage += 1,
            ParseOutcome::Skip(SkipReason::MissingField) => out.skipped.missing_field += 1,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assistant(id: &str, out: u64) -> String {
        format!(
            r#"{{"type":"assistant","timestamp":"2026-10-28T21:14:03.512Z","sessionId":"sess-1","isSidechain":false,"cwd":"/work/demo-app","requestId":"req_1","extra":{{"a":[1,2]}},"message":{{"id":"{id}","model":"model-x","content":[{{"type":"text","text":"hi"}}],"usage":{{"input_tokens":6,"output_tokens":{out},"cache_creation_input_tokens":1520,"cache_read_input_tokens":48210,"iterations":[{{"input_tokens":6}}],"future_field":true}}}}}}"#
        )
    }

    fn record(line: &str) -> UsageRecord {
        match parse_line(line) {
            ParseOutcome::Record(r) => r,
            other => panic!("expected record, got {other:?}"),
        }
    }

    fn skip(line: &str) -> SkipReason {
        match parse_line(line) {
            ParseOutcome::Skip(r) => r,
            other => panic!("expected skip, got {other:?}"),
        }
    }

    #[test]
    fn parses_assistant_line_and_ignores_unknown_fields() {
        let r = record(&assistant("msg_1", 412));
        assert_eq!(r.message_id, "msg_1");
        assert_eq!(r.request_id.as_deref(), Some("req_1"));
        assert_eq!(r.session_id, "sess-1");
        assert_eq!(r.agent_id, None);
        assert!(!r.is_sidechain);
        assert_eq!(r.ts, 1_793_222_043_512);
        assert_eq!(r.model, "model-x");
        assert_eq!(r.project_path, "/work/demo-app");
        assert_eq!(
            (
                r.input_tokens,
                r.output_tokens,
                r.cache_creation_tokens,
                r.cache_read_tokens
            ),
            (6, 412, 1520, 48210)
        );
    }

    #[test]
    fn subagent_fields() {
        let line = r#"{"type":"assistant","timestamp":"2026-10-28T21:14:03Z","sessionId":"s","isSidechain":true,"agentId":"a1","message":{"id":"m","model":"x","usage":{"input_tokens":1}}}"#;
        let r = record(line);
        assert!(r.is_sidechain);
        assert_eq!(r.agent_id.as_deref(), Some("a1"));
        assert_eq!(r.project_path, "");
        assert_eq!(r.output_tokens, 0);
    }

    #[test]
    fn skips_other_line_types() {
        for t in [
            "user",
            "system",
            "cost-state",
            "api-request-blob",
            "brand-new",
        ] {
            let line = format!(
                r#"{{"type":"{t}","message":{{"role":"assistant","id":"m","usage":{{"input_tokens":5}}}}}}"#
            );
            assert_eq!(skip(&line), SkipReason::NotAssistant, "{t}");
        }
    }

    #[test]
    fn other_types_are_skipped_whatever_their_shape() {
        assert_eq!(
            skip(r#"{"type":"user","message":"text"}"#),
            SkipReason::NotAssistant
        );
        assert_eq!(
            skip(r#"{"type":7,"message":[1]}"#),
            SkipReason::NotAssistant
        );
        assert_eq!(skip(r#"{"message":"x"}"#), SkipReason::NotAssistant);
    }

    #[test]
    fn skips_synthetic_lines() {
        let a = r#"{"type":"assistant","timestamp":"2026-10-28T21:14:03Z","sessionId":"s","message":{"id":"uuid","model":"<synthetic>","usage":{"input_tokens":0}}}"#;
        let b = r#"{"type":"assistant","isApiErrorMessage":true,"timestamp":"2026-10-28T21:14:03Z","sessionId":"s","message":{"id":"uuid","model":"x","usage":{"input_tokens":0}}}"#;
        assert_eq!(skip(a), SkipReason::Synthetic);
        assert_eq!(skip(b), SkipReason::Synthetic);
    }

    #[test]
    fn skips_missing_usage_and_fields() {
        let no_usage = r#"{"type":"assistant","timestamp":"2026-10-28T21:14:03Z","sessionId":"s","message":{"id":"m","model":"x"}}"#;
        let no_msg = r#"{"type":"assistant"}"#;
        let no_id = r#"{"type":"assistant","timestamp":"2026-10-28T21:14:03Z","sessionId":"s","message":{"model":"x","usage":{}}}"#;
        let no_session = r#"{"type":"assistant","timestamp":"2026-10-28T21:14:03Z","message":{"id":"m","model":"x","usage":{}}}"#;
        let bad_ts = r#"{"type":"assistant","timestamp":"yesterday","sessionId":"s","message":{"id":"m","model":"x","usage":{}}}"#;
        assert_eq!(skip(no_usage), SkipReason::MissingUsage);
        assert_eq!(skip(no_msg), SkipReason::MissingUsage);
        assert_eq!(skip(no_id), SkipReason::MissingField);
        assert_eq!(skip(no_session), SkipReason::MissingField);
        assert_eq!(skip(bad_ts), SkipReason::MissingField);
    }

    #[test]
    fn malformed_and_blank() {
        assert_eq!(skip(""), SkipReason::Blank);
        assert_eq!(skip("  \r"), SkipReason::Blank);
        assert_eq!(skip("not json"), SkipReason::Malformed);
        assert_eq!(skip("[1,2]"), SkipReason::Malformed);
        let cut = assistant("msg_1", 1);
        assert_eq!(skip(&cut[..cut.len() / 2]), SkipReason::Malformed);
    }

    #[test]
    fn odd_token_values_count_as_zero() {
        let line = r#"{"type":"assistant","timestamp":"2026-10-28T21:14:03Z","sessionId":"s","message":{"id":"m","model":"x","usage":{"input_tokens":-3,"output_tokens":"7","cache_read_input_tokens":2.5,"cache_creation_input_tokens":null}}}"#;
        let r = record(line);
        assert_eq!(
            (
                r.input_tokens,
                r.output_tokens,
                r.cache_creation_tokens,
                r.cache_read_tokens
            ),
            (0, 0, 0, 0)
        );
    }

    #[test]
    fn complete_prefix() {
        assert_eq!(complete_prefix_len(b""), 0);
        assert_eq!(complete_prefix_len(b"abc"), 0);
        assert_eq!(complete_prefix_len(b"abc\n"), 4);
        assert_eq!(complete_prefix_len(b"abc\ndef"), 4);
    }

    #[test]
    fn chunk_consumes_complete_lines_only() {
        let mut buf = String::new();
        buf.push_str(&assistant("msg_1", 10));
        buf.push('\n');
        buf.push_str("garbage\n\n");
        buf.push_str(r#"{"type":"user"}"#);
        buf.push('\n');
        buf.push_str(&assistant("msg_2", 20));
        buf.push('\n');
        let partial = assistant("msg_3", 30);
        let full_len = buf.len();
        buf.push_str(&partial[..40]);

        let res = parse_chunk(buf.as_bytes());
        assert_eq!(res.consumed, full_len);
        assert_eq!(res.records.len(), 2);
        assert_eq!(res.skipped.malformed, 1);
        assert_eq!(res.skipped.not_assistant, 1);
        assert_eq!(res.skipped.total(), 2);

        // Retry from the offset once the line is complete.
        let mut rest = buf.as_bytes()[res.consumed..].to_vec();
        rest.extend_from_slice(&partial.as_bytes()[40..]);
        rest.push(b'\n');
        let res2 = parse_chunk(&rest);
        assert_eq!(res2.records.len(), 1);
        assert_eq!(res2.records[0].message_id, "msg_3");
        assert_eq!(res2.consumed, rest.len());
    }

    #[test]
    fn chunk_survives_invalid_utf8_and_crlf() {
        let mut buf = assistant("msg_1", 1).into_bytes();
        buf.extend_from_slice(b"\r\n\xff\xfe\n");
        let res = parse_chunk(&buf);
        assert_eq!(res.records.len(), 1);
        assert_eq!(res.skipped.malformed, 1);
    }
}
