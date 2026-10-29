//! Database schema contract. The SQL is the source of truth for migration 1;
//! see `schema_v1.sql` and docs/ARCHITECTURE.md ("Storage").

/// Schema version the app expects after all migrations.
pub const SCHEMA_VERSION: u32 = 1;

/// SQL of schema version 1.
pub const SCHEMA_V1_SQL: &str = include_str!("schema_v1.sql");
