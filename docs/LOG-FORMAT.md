# Claude Code log format

What Tokenmonk reads, as observed in local Claude Code logs (Claude Code
2.1.x, October 2026, Windows, desktop entry point). The format is not a
public API and changes between releases: the parser takes only the fields
listed here, ignores everything else and never fails on unknown line types.

All example values below are invented.

## Location and layout

Root: `~/.claude/projects/` (on Windows `%USERPROFILE%\.claude\projects\`).
Tokenmonk only reads; it never writes, moves or deletes anything there.

```
~/.claude/projects/
  <project-dir>/                          one folder per working directory
    <session-uuid>.jsonl                  main conversation of one session
    <session-uuid>/
      subagents/
        agent-<agent-id>.jsonl            one subagent (sidechain) run
        agent-<agent-id>.meta.json        not a log, ignore
      tool-results/*.txt                  not a log, ignore
    <session-uuid>.<something>.json       not a log, ignore
  <project-dir>/memory/*.md               not a log, ignore
```

- Read every `*.jsonl` below the root, recursively (main files at depth 2,
  subagent files at depth 4). Ignore all other files.
- `<project-dir>` is the working directory with every non-alphanumeric
  character replaced by `-` (for example `-Users-example-dev-demo-app`). The
  encoding is lossy, so the project path comes from the `cwd` field of each
  line, never from the folder name. The `cwd` can change inside one file.
- Files are append-only JSON Lines, UTF-8, LF line endings, one JSON object
  per line, written while Claude Code runs.

## Line types

Every line has a `type`. Observed types:

| `type` | Use |
|---|---|
| `assistant` | **The only source of token usage.** One line per content block of an API response. |
| `user` | Prompts and tool results. No usage. Ignored for tokens. |
| `system` | Hooks, compaction boundaries, API errors (`subtype`). Ignored. |
| `attachment`, `queue-operation`, `last-prompt`, `custom-title`, `ai-title`, `mode`, `agent-name`, `agent-setting`, `pr-link`, `file-history-snapshot`, `file-history-delta`, `bridge-session`, `atis-latch` | Session metadata. Ignored. |
| `cost-state` | Session cost summary. Ignored: it would double count. |
| `api-request`, `api-request-shape`, `api-request-blob` | Request bookkeeping. Ignored (`api-request-blob` even carries a `message.role` of `assistant`; only `type` decides). |

Any other `type` is skipped silently.

## Fields used from `assistant` lines

| Field | Type | Meaning |
|---|---|---|
| `type` | string | `"assistant"` |
| `timestamp` | string | ISO 8601 in UTC with milliseconds and `Z`, e.g. `2026-10-28T21:14:03.512Z` |
| `sessionId` | string (UUID) | Session the message belongs to |
| `isSidechain` | bool | `true` in subagent files, `false` in main files |
| `agentId` | string | Present in subagent files |
| `cwd` | string | Working directory = project path |
| `requestId` | string | API request id (`req_...`); absent on synthetic lines |
| `message.id` | string | API message id (`msg_...`); **dedup key** |
| `message.model` | string | Model id, e.g. `claude-sonnet-5`; `<synthetic>` for client-made lines |
| `message.usage.input_tokens` | number | Uncached input tokens |
| `message.usage.output_tokens` | number | Output tokens (grows across the lines of one message) |
| `message.usage.cache_creation_input_tokens` | number | Tokens written to the prompt cache |
| `message.usage.cache_read_input_tokens` | number | Tokens read from the prompt cache |
| `isApiErrorMessage` | bool | Present and `true` on synthetic error lines |

Other `usage` members exist and are not needed: `cache_creation`
(`ephemeral_5m_input_tokens` + `ephemeral_1h_input_tokens`, always summing to
`cache_creation_input_tokens`), `iterations` (a per-iteration breakdown
whose sums equal the top-level counts, so never add it on top),
`server_tool_use`, `output_tokens_details`, `service_tier`, `speed`,
`inference_geo`, `fallback_credit`. No line carries a cost in USD.

Example (invented, shortened, pretty-printed; on disk it is one line):

```json
{
  "type": "assistant",
  "uuid": "11111111-2222-4333-8444-555555555555",
  "parentUuid": "66666666-7777-4888-9999-000000000000",
  "sessionId": "0f0e0d0c-0b0a-4908-8706-050403020100",
  "isSidechain": false,
  "cwd": "/Users/example/dev/demo-app",
  "gitBranch": "main",
  "version": "2.1.0",
  "timestamp": "2026-10-28T21:14:03.512Z",
  "requestId": "req_000000000000000000000001",
  "message": {
    "id": "msg_000000000000000000000001",
    "type": "message",
    "role": "assistant",
    "model": "claude-sonnet-5",
    "content": [{ "type": "text", "text": "(content not read by Tokenmonk)" }],
    "stop_reason": "end_turn",
    "usage": {
      "input_tokens": 6,
      "cache_creation_input_tokens": 1520,
      "cache_read_input_tokens": 48210,
      "output_tokens": 412,
      "cache_creation": { "ephemeral_5m_input_tokens": 0, "ephemeral_1h_input_tokens": 1520 }
    }
  }
}
```

Tokenmonk never stores or reads `message.content`.

## Deduplication

The same API message appears on several lines, and in several files:

1. **One line per content block.** A response with thinking, text and two
   tool calls is written as four `assistant` lines with the same
   `message.id` and `requestId` (about three in four messages repeat; up to
   35 lines were seen). On these lines `input_tokens`,
   `cache_creation_input_tokens` and `cache_read_input_tokens` are
   identical, and `output_tokens` never decreases; the last line carries the
   final count. Earlier lines may have `stop_reason: null`.
2. **Copied history.** A resumed or forked session starts a new file and
   copies earlier lines into it unchanged, including their original
   `sessionId`, `uuid` and `message.id`. So a file can contain lines whose
   `sessionId` differs from its file name, and one `message.id` can occur in
   several main files.

Rule: one row per `message.id`, global across all files. On a repeat keep
the first `timestamp`, `sessionId` and `cwd`, and take the maximum of each
token field (in practice only `output_tokens` changes). Because lines arrive
incrementally, this is an upsert, not a read-ahead.

## Lines to skip for tokens

- `message.model == "<synthetic>"` or `isApiErrorMessage == true`: lines
  made by the client (for example after HTTP 429), all usage zero, and the
  `message.id` is a UUID instead of `msg_...`.
- Lines without `message.usage` or without `message.id`.

Observed side note: synthetic 429 lines can carry a `quotaLimits` object
(`status`, `rateLimitType` such as `five_hour`, `resetsAt` in Unix
**seconds**, overage fields). It says that a limit was hit and when it
resets, but carries no utilisation percentage. It is not used in v0.1.

## Sessions and sidechains

- `sessionId` identifies a Claude Code session. A main file
  `<session-uuid>.jsonl` holds that session (plus copied lines, see above).
- Subagent files carry the **parent's** `sessionId`, `isSidechain: true`
  and an `agentId`. Their usage is real API usage and counts towards the
  parent session and all totals. No message id was seen in both a main and a
  subagent file.
- Sessions can last for days (resumed sessions); gaps between consecutive
  assistant messages are mostly under 5 minutes, rarely over an hour.
  Tokenmonk's "This session" is defined in docs/ARCHITECTURE.md.

## Timestamps

- Always UTC (`Z`) with milliseconds. Converted to Unix epoch ms on parse;
  local-time logic (today, night hours, Halloween) happens later.
- Within one file timestamps were monotonic, but copied history makes a
  file's first lines older than its file creation. Never assume order across
  files.

## Partial and malformed lines

- No malformed line was found locally, and every file ended with a newline.
  Files are, however, appended while Tokenmonk reads them, so the last line
  can be incomplete at read time.
- Reader rule: consume only up to the last `\n`; keep the byte offset of the
  first unconsumed byte and retry the rest on the next change.
- A complete line that is not valid JSON, or lacks a required field, is
  skipped and counted, never fatal.
- If a file shrinks below the stored offset or its modification time goes
  back, it was replaced: re-read it from offset 0 (dedup makes that safe).
