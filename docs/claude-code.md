# Claude Code integration

Job Hunter does not call the Anthropic API. It spawns the user's own `claude` executable in non-interactive print mode and reads its structured output.

## Detection

`claude::discover::find_executable` checks, in order: the path from Settings / `CLAUDE_CLI_PATH`, `claude` (`claude.exe`, `claude.cmd`) on PATH, then well-known locations (`~/.local/bin`, `~/.claude/local`, `~/.npm-global/bin`, `%APPDATA%\npm`, nvm directories, Homebrew, `/usr/local/bin`).

On Windows the npm shim `claude.cmd` is unwrapped to the real `claude.exe` (or `node cli.js`) so the process can be spawned with safe argument quoting and killed reliably.

`claude --version` and `claude auth status` (JSON) provide the version and login state shown in the setup wizard.

## Invocation

Every AI task is one `ClaudeRequest`, executed by `CliClaudeRunner`:

```
claude -p \
  --output-format stream-json --verbose \
  --permission-mode dontAsk \
  --strict-mcp-config \
  --max-turns <n> \
  [--chrome | --no-chrome] \
  [--tools ""]                       # disable built-in tools (Bash, Edit, ...)
  [--allowedTools mcp__claude-in-chrome__navigate,...]
  [--model <alias>] [--max-budget-usd <x>] \
  [--json-schema '<schema>'] \
  --append-system-prompt-file <run-dir>/system-prompt.md \
  [--resume <session-id>]
```

- The prompt (task + skill text + JSON inputs) is written to stdin.
- `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` are removed from the child environment so the user's Claude Code login is always used.
- `--permission-mode dontAsk` denies anything not pre-approved, which in print mode means only the allow-listed Chrome tools can run.
- The working directory is a per-run folder under `runs/<run-id>/` in the app data directory. It receives the prompt (`<label>-prompt.md`) and the raw redacted NDJSON transcript (`<label>-<timestamp>.ndjson`) for debugging.

## Streaming protocol

`claude::protocol::StreamParser` consumes the `stream-json` lines:

| type | used for |
| --- | --- |
| `system` / `init` | session id, tool count |
| `assistant` | text blocks and `tool_use` blocks → activity feed ("Opening https://…", "Filling a form field") |
| `user` | tool results (errors surface as warnings) |
| `result` | `structured_output`, `result` text, `is_error`, `subtype`, `session_id`, `total_cost_usd`, `permission_denials` |

If `structured_output` is missing, the parser recovers the first JSON object from the result text. Timeouts and cancellation kill the process.

## Tasks and schemas

| task label | tools | schema |
| --- | --- | --- |
| `discover:<source>` | Chrome read-only | `agent/schemas/discovery.json` |
| `extract:<company>` | Chrome read-only | `extraction.json` |
| `analyze` | none | `analysis.json` |
| `generate_resume:<company>` | none | `resume.json` |
| `validate_resume:<company>` | none | `validation.json` |
| `apply:<company>` | Chrome read-only + `form_input` + `file_upload` | `apply.json` |
| `parse_profile` | none | `profile-parse.json` |

## Session resume

Application runs record `session_id`. When the user answers pending questions, the apply task is resumed with `--resume <session-id>` so Claude can continue in the same tab; if the session cannot be resumed the task restarts from the job URL.

## Cost and limits

Settings expose `--max-budget-usd` per call (default 3.00), max browser turns (default 80) and a per-call timeout (default 15 minutes). Costs reported by Claude are summed into the run statistics shown on the Agent page.
