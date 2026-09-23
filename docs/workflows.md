# Workflows

## Job hunt

```
Start Job Hunt
  → preflight (Claude, Chrome, profile)
  → discover per source (Claude in Chrome, read-only)
  → extract missing details
  → deduplicate (deterministic)
  → analyze (batches of 4)
  → prepare applications for relevant jobs above the minimum score
       generate resume → validate → revise (≤ 3) → render DOCX/PDF → cover letter
       application READY_FOR_REVIEW with profile + known answers and potential issues
  → WAITING_FOR_APPROVAL
```

## Approval

```
READY_FOR_REVIEW ──Approve & Apply──► APPROVED ──► APPLYING
                 ──Reject───────────► REJECTED (can be reopened to review)
```

Approval is recorded with `approvedAt`; nothing else can move an application into `APPLYING`.

## Browser application

```
APPLYING
  ├─ SUBMITTED + evidence ───────────► APPLIED
  ├─ SUBMITTED without evidence ─────► MANUAL_ACTION_REQUIRED ("could not verify")
  ├─ HUMAN_INPUT_REQUIRED ───────────► WAITING_FOR_USER
  │       user answers in the app (answers stored for reuse) → APPLYING (session resumed)
  ├─ MANUAL_ACTION_REQUIRED ─────────► MANUAL_ACTION_REQUIRED (reason, deepest URL, steps done)
  └─ FAILED / Claude error / stop ───► MANUAL_ACTION_REQUIRED
```

## Manual fallback

`MANUAL_ACTION_REQUIRED` shows the reason, **Open Application** (deepest URL reached), the resume/cover letter files, the known answers and **Mark as Applied**. **Retry automatically** is offered for approved applications.

## Tracking

`APPLIED → INTERVIEW → OFFER`, or `REJECTED` / `WITHDRAWN` at any point. Every change is appended to `statusHistory` with a reason and timestamp; notes are free text.

## Single-job actions

From a job's detail page: **Analyze** / **Re-analyze**, **Prepare application** (generates materials and creates the review record), **Reject**, **Save**, **Open Original Job**.
