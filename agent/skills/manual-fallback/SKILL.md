# Skill: manual-fallback

When automatic application is not possible, the user completes it by hand. Make that easy:

- `reason` must be specific and actionable ("The employer site shows a Cloudflare 'verify you are human' check before the form", not "blocked").
- `applicationUrl` must be the deepest URL you reached (the form page, not the listing) so the user can continue where you stopped.
- `stepsCompleted` lists what was already filled so the user does not repeat it.
- If you uploaded the resume before being blocked, say so.
- Leave the tab open unless told otherwise.

The desktop application then shows **Apply Manually** (opens the URL in Chrome) alongside the generated resume, cover letter and known answers, and lets the user press **Mark as Applied** afterwards.
