# Architecture

Job Hunter is a single desktop process. There is no server.

```
┌──────────────────────────────────────────────────────────────┐
│  Tauri window (WebView)                                      │
│  React + Chakra UI + TanStack Query/Router                   │
│  invoke("command") ◄──► events: agent:event, agent:status,   │
│                                  sync:status, log:entry,     │
│                                  data:changed                │
├──────────────────────────────────────────────────────────────┤
│  apps/desktop/src-tauri  (Rust)                              │
│  thin commands → job_hunter_core::AppContext                 │
├──────────────────────────────────────────────────────────────┤
│  crates/job-hunter-core (Rust, no Tauri dependency)          │
│                                                              │
│  domain/        entities (camelCase serde, shared with TS)   │
│  store/         LocalStore (JSON per collection) + SyncWorker│
│                 → MongoDB Atlas (upsert by id, newest wins)  │
│  claude/        CLI discovery, stream-json runner, mock      │
│  chrome/        Chrome + extension detection, print-to-pdf   │
│  documents/     PDF/DOCX import, HTML/DOCX/PDF rendering     │
│  dedup.rs       canonical URL + fuzzy company/title/desc     │
│  prompts.rs     embeds agent/ files, builds per-task prompts │
│  agent/         state machine, event bus, steps, orchestrator│
│  context.rs     AppContext: the command surface              │
└──────────────────────────────────────────────────────────────┘
          │                                  │
          ▼                                  ▼
   claude -p (stream-json)            MongoDB Atlas (optional)
          │
          ▼
   Claude in Chrome ─► Chrome ─► job sites
```

## Core decisions

**Local-first persistence.** `LocalStore` keeps one JSON file per collection in the app data directory (`%APPDATA%\JobHunter\database` on Windows) with an in-memory cache, atomic writes and a persisted outbound queue. Every write is queued for Atlas; `SyncWorker` flushes the queue when Atlas is reachable and pulls remote documents on start, merging by `updatedAt`. MongoDB being down never blocks the UI or the agent.

**Claude Code as the only AI engine.** Every AI step is one non-interactive process: `claude -p --output-format stream-json --verbose --permission-mode dontAsk --strict-mcp-config --max-turns N [--chrome --allowedTools …] [--json-schema …] --append-system-prompt-file system-prompt.md`. The prompt is written to stdin; the structured result is read from the final `result` message (`structured_output`). The child inherits the user's Claude login; `ANTHROPIC_API_KEY` is explicitly removed from its environment. See [claude-code.md](claude-code.md).

**Browser work is delegated to Claude in Chrome.** The Rust side never drives the browser itself. It passes `--chrome` and a strict `--allowedTools` list of `mcp__claude-in-chrome__*` tools: read-only tools for discovery/extraction, plus `form_input` and `file_upload` for approved applications. Built-in tools (Bash, Edit, …) are disabled with `--tools ""`.

**Explicit state machine.** `agent/state.rs` defines the allowed transitions (`IDLE → INITIALIZING → DISCOVERING → EXTRACTING → DEDUPLICATING → ANALYZING → PREPARING_APPLICATIONS → WAITING_FOR_APPROVAL → APPLYING → COMPLETED`, plus `PAUSED`, `STOPPING`, `FAILED`, `MANUAL_ACTION_REQUIRED`, `WAITING_FOR_USER`). `AgentHandle` owns the live status, the cancellation token and the pause flag; steps call `checkpoint()` between units of work so pause/stop take effect promptly. Each step persists its output before the next starts.

**Approval is structural, not a prompt.** `Application::transition` only allows `APPLYING` from `APPROVED` (or a user-resumed `WAITING_FOR_USER`), `approve_application` is the only path that sets `approvedAt`, and the apply prompt states `APPROVED = true|false` explicitly. `record_apply_result` refuses to mark `APPLIED` when Claude reports `SUBMITTED` without evidence.

**Deterministic rendering.** Claude returns a structured `ResumeDocument`; Rust renders HTML, DOCX (docx-rs) and PDF (headless Chrome `--print-to-pdf`, with a bundled-font fallback renderer). Versions are stored under `resumes/generated/<company-slug>/resume-vN.*`.

**Mock mode.** `MOCK_MODE=true` (or the Settings toggle) swaps `CliClaudeRunner` for `MockClaudeRunner`, which answers from `agent/fixtures` with deterministic heuristics and never reports a submission. The same runner powers the end-to-end tests in `crates/job-hunter-core/tests/mock_pipeline.rs`.

## Data flow of one job hunt

1. `start_job_hunt` claims the agent, persists an `AgentRun`, spawns the pipeline task.
2. `preflight` checks Claude CLI + auth, Chrome + extension, profile completeness.
3. For each enabled source: `discover_source` (Claude + Chrome, discovery schema) → `Job`s.
4. `extract_details` for jobs without a full description.
5. `dedup::deduplicate` against all stored jobs; merged duplicates gain a second `sources` entry.
6. `analyze_jobs` in batches of four (Claude, no tools, analysis schema) → `JobAnalysis`; job status becomes `SHORTLISTED`, `ANALYZED` or `NOT_RELEVANT`.
7. For relevant jobs above `minimumMatchScore` (up to `maxApplicationsPerRun`): `generate_resume` (generate → validate → revise, ≤ 3) → render → `prepare_application` → `READY_FOR_REVIEW`.
8. The run ends in `WAITING_FOR_APPROVAL`; the UI shows "N applications waiting for your approval".

## Events

`AgentEvent { runId, at, level, kind, message, data }` flows through `EventBus` to the Tauri layer, which emits `agent:event`. Persistent kinds (`STATE_CHANGED`, `STEP_*`, `JOB_*`, `RESUME_GENERATED`, `APPLICATION_*`, `RUN_FINISHED`) are also stored in `agent_events`; high-frequency `CLAUDE_ACTIVITY` events are live-only.
