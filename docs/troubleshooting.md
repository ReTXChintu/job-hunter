# Troubleshooting

| Symptom | Cause / fix |
| --- | --- |
| "Claude CLI is not installed or could not be found" | Install Claude Code; make sure `claude --version` works in a fresh terminal. Set the explicit path in Settings → Claude & Chrome if it is installed somewhere unusual. |
| "Claude CLI is installed but not signed in" | Run `claude auth login` (or `claude` and complete the browser login). `claude auth status` must show `loggedIn: true`. |
| "Claude in Chrome is not available" | Install the Claude extension in Chrome, enable it, sign in, and keep Chrome open. Then re-check in the setup wizard. Not supported in WSL. |
| Discovery finds 0 jobs / "site blocked the search" | The site showed a login wall, CAPTCHA or an "unusual activity" notice. Sign in to the site in Chrome, wait, and try again; reduce `Max jobs per source`. |
| "Claude did not finish within N seconds" | Increase the timeout or reduce `Max jobs per source` / `Max browser turns`. Check the run transcript under `runs/<run-id>/` in the data directory. |
| "Claude hit the budget limit" | Raise `Max spend per Claude call` in Settings or set it to 0 (unlimited). |
| Resume PDF says "built-in renderer" | Chrome headless export failed (Chrome missing or busy). The fallback PDF is still ATS-friendly; install Chrome or set its path to restore the higher-fidelity output. |
| "The backend is unreachable" | Check the backend server is running and reachable, and the address in Settings → Backend account is correct. Data is safe locally and syncs when the connection returns; pending count is shown in the status bar. |
| "The candidate profile is incomplete" | Name, email, at least one target role and some skills are required. |
| Application stuck in "Applying" after a crash | Restart the app; the run is finalised as manual action. Open the application and use Retry automatically or Apply Manually. |
| Nothing happens when clicking buttons in the browser | You opened the Vite dev server in a browser. Job Hunter must run inside Tauri (`pnpm tauri dev`). |

## Where to look

- **Agent → Logs** (or Settings → Logs): structured logs with Copy / Export / Clear.
- **Agent → Runs**: persisted events per run.
- `runs/<run-id>/<label>-prompt.md` and `<label>-<timestamp>.ndjson`: the exact prompt and the redacted Claude transcript for each task.
- `logs/job-hunter-YYYY-MM-DD.log`: daily log files.

## Known limitations

- V1 discovers through browser interaction on LinkedIn, Naukri, Indeed and Wellfound; company career pages, Greenhouse, Lever and Workday are supported for extraction and application but not searched automatically.
- Discovery and application quality depend on the site's current layout, your login state and the Claude model in use.
- Multi-step assessments, video interviews, account creation and paid services are always handed over to you.
- Cover letters are generated together with the resume; regenerating one regenerates both (new version).
- Backend sync is last-writer-wins by `updatedAt`; there is no conflict UI.
- Single user per installation (schemas carry `userId` for future multi-user support).
- The Claude in Chrome check is static (extension files present); a disabled extension is only detected when a task starts.

## Next recommended improvements

1. Search recipes for Greenhouse/Lever/Workday boards and configurable company career URLs.
2. Daily summary report (jobs found, applications sent, follow-ups due).
3. Answer database editor with categories and suggested answers from the profile.
4. Per-source rate limiting and scheduling ("run every morning").
5. Conflict-aware sync and multi-device usage.
6. Signed installers and auto-update.
