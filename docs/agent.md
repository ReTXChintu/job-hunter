# The agent

## State machine

```
IDLE → INITIALIZING → DISCOVERING → EXTRACTING → DEDUPLICATING → ANALYZING
     → PREPARING_APPLICATIONS → WAITING_FOR_APPROVAL → APPLYING → COMPLETED

side states: PAUSED (from any working state, resumes to the same state),
             STOPPING (after Stop; ends in COMPLETED with what was gathered),
             FAILED, MANUAL_ACTION_REQUIRED, WAITING_FOR_USER
```

Transitions are validated by `agent::state::can_transition`; an invalid transition is a bug and returns `InvalidTransition` instead of silently proceeding.

## Runs

Every start creates an `AgentRun` (`JOB_HUNT`, `ANALYSIS`, `RESUME_GENERATION` or `APPLICATION`) with statistics (jobs discovered/new/duplicates/analyzed/relevant/awaiting approval/applied/manual/errors, Claude cost). Only one run executes at a time; starting another returns `AgentBusy`.

## Commands

| command | effect |
| --- | --- |
| `start_job_hunt` | full pipeline (optionally `discoverOnly`, restricted `sources`) |
| `stop_job_hunt` | cancels the current Claude process; the run completes with what it has |
| `pause_agent` / `resume_agent` | pauses between units of work |
| `discover_jobs` | discovery + analysis only |
| `analyze_job` | (re)analyse one job |
| `generate_resume` / `generate_cover_letter` | regenerate materials for one job → application in review |
| `approve_application` | explicit approval, then starts the browser application |
| `reject_application` | reject (reopenable to review) |
| `apply_application` | run the browser application for an approved record |
| `answer_application_questions` | store answers, resume the application |
| `mark_manual_application_complete` | user finished manually → `APPLIED` |
| `set_application_status` | tracking: `INTERVIEW`, `OFFER`, `WITHDRAWN`, `REJECTED` |

## Events

`AgentEvent.kind` values: `STATE_CHANGED`, `STEP_STARTED`, `STEP_DONE`, `STEP_FAILED`, `PROGRESS`, `MESSAGE`, `JOB_DISCOVERED`, `JOB_ANALYZED`, `RESUME_GENERATED`, `APPLICATION_READY`, `APPLICATION_UPDATED`, `CLAUDE_ACTIVITY`, `RUN_FINISHED`. Levels: `INFO`, `SUCCESS`, `WARN`, `ERROR`.

The UI keeps the last 600 events in memory for the live feed; persisted events are queryable per run on the Agent page.

## Instructions and skills

- `agent/system/job-hunter.md` is appended to Claude's system prompt on every task.
- Each task embeds the relevant `agent/skills/*/SKILL.md` text in the prompt plus JSON inputs.
- Outputs are constrained with `agent/schemas/*.json` via `--json-schema`.
- All of these files are embedded into the binary at compile time (`prompts.rs`), so edits require a rebuild.

## Resumability

Each step persists before the next begins: discovered jobs are saved after de-duplication, analyses immediately, resumes and applications individually. A crashed run leaves usable data; the next run skips jobs whose URL is already known.
