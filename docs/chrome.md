# Chrome and Claude in Chrome

Job Hunter never automates the browser itself. Claude Code, started with `--chrome`, talks to the **Claude in Chrome** extension over native messaging and operates a tab group inside your running Chrome. Because it is your real profile, job sites see your normal logged-in session.

## Requirements

- Google Chrome (or another Chromium browser supported by the extension).
- Claude in Chrome extension 1.0.36+ (Chrome Web Store id `fcoeoabgfenejglbffodgkkbkcdhcgfn`), installed, enabled and signed in.
- Claude Code 2.0.73+ signed in with an Anthropic plan.
- Chrome running while the agent works (start it before a job hunt).
- Not supported inside WSL.

## Detection

`chrome::status` finds the Chrome executable (standard install paths, PATH, or the override in Settings) and looks for the extension in every Chrome profile (`Default`, `Profile *`) under the user data directory. This is a static check; the live connection is established by Claude Code when a task starts.

## Tools the agent may use

Read-only (discovery, extraction):

```
list_connected_browsers, tabs_context_mcp, tabs_create_mcp, tabs_close_mcp,
navigate, get_page_text, read_page, find, computer, browser_batch, resize_window
```

Approved applications add:

```
form_input, file_upload
```

Everything else (JavaScript execution, GIF recording, image upload, Claude Code's own Bash/Edit tools) is disabled.

## Rules the browser skill enforces

From `agent/skills/chrome-application/SKILL.md`:

- never submit unless the task states the application is approved;
- do not bypass CAPTCHA or anti-bot protection; stop and report `MANUAL_ACTION_REQUIRED`;
- do not enter data that is not in the inputs; report unknown questions as `HUMAN_INPUT_REQUIRED`;
- never claim success without visible evidence; quote the confirmation;
- never read passwords, cookies or session tokens; stay on the job's site.

## Headless PDF export

Independently of the extension, Job Hunter uses `chrome --headless=new --print-to-pdf` with a throw-away `--user-data-dir` to turn the generated HTML resume into a PDF. This never touches your running browser profile. If Chrome is unavailable, a bundled-font PDF renderer is used instead.

## Limits

- Sites that require a fresh login, phone verification, or show "unusual activity" pages stop the task; you finish manually.
- Multi-step assessments, video questions and external ATS portals that require account creation are handed over to you.
- Search result layouts change; the discovery skill uses URL parameters where possible but quality varies by site and model.
