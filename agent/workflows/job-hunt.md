# Workflow: job hunt

Started manually by the user with **Start Job Hunt**. Orchestrated by the Rust agent; each step is one Claude Code invocation unless noted.

```
IDLE
 ↓ start_job_hunt
INITIALIZING        check Claude CLI, auth, Chrome, profile completeness; create run
 ↓
DISCOVERING         per enabled source: job-discovery skill (Claude + Chrome)
 ↓
EXTRACTING          per job missing details: job-extraction skill (Claude + Chrome)
 ↓
DEDUPLICATING       deterministic (Rust): canonical URL, ids, company/title/location, description similarity
 ↓
ANALYZING           batches of jobs: job-analysis skill (Claude, no tools)
 ↓
PREPARING_APPLICATIONS
                    for relevant jobs up to maxApplicationsPerRun:
                    resume-generation → resume-validation → (revise ≤ 3) → render DOCX/PDF → application READY_FOR_REVIEW
 ↓
WAITING_FOR_APPROVAL
                    the user reviews each application in the desktop app
 ↓ approve_application (per application)
APPLYING            chrome-application skill (Claude + Chrome) — see application.md
 ↓
COMPLETED / MANUAL_ACTION_REQUIRED / WAITING_FOR_USER / FAILED
```

Each step persists its output before the next starts, so a crashed run leaves usable jobs, analyses and resumes behind. `stop_job_hunt` cancels the current Claude process and marks the run `COMPLETED` with what was gathered; `pause_agent` finishes the current step and waits.
