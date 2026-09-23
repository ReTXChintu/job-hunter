# Skill: application-tracking

Application status is maintained by the desktop application from your reports.

| Your outcome | Resulting status |
| --- | --- |
| `SUBMITTED` (with evidence) | `APPLIED` |
| `HUMAN_INPUT_REQUIRED` | `WAITING_FOR_USER` (the user answers in the app, then the task resumes) |
| `MANUAL_ACTION_REQUIRED` | `MANUAL_ACTION_REQUIRED` (the user finishes in the browser and marks it applied) |
| `FAILED` | `MANUAL_ACTION_REQUIRED` with the failure reason |

Every report must include `stepsCompleted` so the user can see how far the automatic application got, and `applicationUrl` (the final page you were on) so they can continue from there.
