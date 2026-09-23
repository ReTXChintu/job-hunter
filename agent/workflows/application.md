# Workflow: application

Triggered only by the user's explicit **Approve & Apply**.

```
READY_FOR_REVIEW
 ↓ approve_application            (user action, recorded with approvedAt)
APPROVED
 ↓ apply_application
APPLYING                          chrome-application skill with APPROVED = true
 │
 ├─ SUBMITTED (with evidence) ──────────────→ APPLIED
 ├─ HUMAN_INPUT_REQUIRED ───────────────────→ WAITING_FOR_USER
 │      user answers in the app (answers are stored for reuse)
 │      ↓ continue → APPLYING (resumes the same Claude session when possible)
 ├─ MANUAL_ACTION_REQUIRED / FAILED ───────→ MANUAL_ACTION_REQUIRED
 │      [Apply Manually] opens the URL; [Mark as Applied] → APPLIED
 └─ reject_application (any time) ─────────→ REJECTED
```

The agent never re-runs an application that is `APPLIED`, `REJECTED` or `WITHDRAWN`, never applies to a job the user rejected, and never applies to a duplicate of a job that already has an application.
