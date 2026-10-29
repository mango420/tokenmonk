//! Claude Code log discovery and parsing (read only).
//!
//! Everything here is pure: bytes and lines in, records out. Reading files,
//! offsets and watching live in the watcher; storage lives in `db`.
//! Format reference: docs/LOG-FORMAT.md.

mod parse;

pub use parse::{
    complete_prefix_len, parse_chunk, parse_line, ChunkResult, ParseOutcome, SkipCounts,
    SkipReason, UsageRecord,
};
