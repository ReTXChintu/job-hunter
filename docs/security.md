# Security

## What Job Hunter never does

- Never uses or stores an Anthropic API key. `ANTHROPIC_API_KEY` / `ANTHROPIC_AUTH_TOKEN` are removed from the Claude child process environment.
- Never reads Claude Code's credentials; it only runs `claude auth status`.
- Never reads browser password stores, cookies or session tokens. The browser is operated by Claude in Chrome inside your normal profile; Job Hunter only receives Claude's structured report.
- Never stores the MongoDB password in a file: the URI lives in the OS credential store (`keyring`). Log lines and error messages have credentials redacted (`secrets::redact_secrets`).
- Never submits an application without the explicit `Approve & Apply` action, and never records `APPLIED` without evidence.
- Never bypasses CAPTCHA or anti-bot protection.

## Process sandboxing

Claude runs with `--permission-mode dontAsk`, `--tools ""` (no Bash/Edit/Read) and an explicit `--allowedTools` list. Read-only tasks cannot fill forms; apply tasks add only `form_input` and `file_upload`. `--strict-mcp-config` prevents the user's other MCP servers from being loaded.

## Files

- App data directory: `%APPDATA%\JobHunter` (Windows), `~/Library/Application Support/JobHunter` (macOS), `~/.local/share/job-hunter` (Linux).
- Contains: `settings.json` (non-secret), `database/*.json`, `resumes/`, `cover-letters/`, `runs/<id>/` (prompts + redacted transcripts), `logs/`, `exports/`, `temp/`.
- The desktop UI can only read/open files inside this directory (`read_text_file`, `open_path` enforce the prefix).
- `.env` is git-ignored; only `.env.example` is committed.

## Content Security Policy

The Tauri window uses a strict CSP (`default-src 'self'`); resume previews are rendered in a fully sandboxed `iframe` (`sandbox=""`) from `srcdoc`, so no script in generated HTML can execute.

## Logging

Structured logs (DEBUG/INFO/WARN/ERROR) are kept in memory (5,000 entries), in daily files under `logs/`, and streamed to the in-app viewer with Copy / Export / Clear. Secrets are redacted before an entry is stored.
