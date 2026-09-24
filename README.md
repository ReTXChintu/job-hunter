# Job Hunter

A local-first desktop job-hunting command center. Job Hunter discovers recent jobs, analyses them against your real profile, generates truthful ATS-friendly resumes and cover letters, and, only after you press **Approve & Apply**, fills the application in your own Chrome through Claude in Chrome.

It runs entirely on your machine. AI work goes through your already-authenticated **Claude Code / Claude CLI** installation. There is no Anthropic API key anywhere in this project.

```
Job Hunter Desktop App (Tauri + React)
        │
        ▼
Rust agent runtime (state machine, orchestration, persistence)
        │
        ▼
claude -p  (your signed-in Claude Code)
        │
        ▼
Claude ──► Claude in Chrome ──► Google Chrome ──► job websites
```

Data is stored locally under your app data directory and mirrored to MongoDB Atlas whenever it is reachable.

An optional **mobile companion app** (Flutter) lets you review and approve/reject applications from your phone. It runs no AI and holds no job data of its own — it's a thin review client that reaches your desktop through a small relay service you self-host, only when you choose to pair it. See [docs/mobile.md](docs/mobile.md) and [docs/relay.md](docs/relay.md).

A self-hosted **Node backend** (`apps/backend`) is also in the repo, built and tested but not yet wired in: it will let the mobile app read jobs and applications directly even when the desktop is offline, and eventually become the one thing that talks to MongoDB Atlas, with the desktop syncing to it the same local-first way it syncs to Atlas today. See [docs/backend.md](docs/backend.md) for what's built and what's still ahead.

## What it does

1. Import your master resume (PDF/DOCX) and build a candidate profile.
2. **Start Job Hunt**: Claude searches your enabled sources (LinkedIn, Naukri, Indeed, Wellfound, …) in Chrome, extracts full postings, and Job Hunter de-duplicates them.
3. Every job is analysed (match score, matched/missing skills, concerns) using only your profile as the source of truth.
4. For relevant jobs it generates a tailored resume, validates it for ATS coverage and unsupported claims (max 3 revise loops), renders DOCX + PDF, and writes a cover letter.
5. Applications land in **Awaiting approval**. You review the job, analysis, resume and cover letter.
6. **Approve & Apply** opens the posting in Chrome and Claude fills the form. It stops for questions it cannot answer truthfully, CAPTCHAs, logins or anti-bot walls, and never claims success without visible evidence.
7. Anything it cannot finish becomes **Manual action required** with a reason, the deepest URL reached, the documents and known answers, plus **Mark as Applied**.
8. Every job, analysis, resume, cover letter, application, answer, run and event is persisted locally and synced to Atlas.

## Requirements

| Requirement | Notes |
| --- | --- |
| Node.js 20+ and pnpm 12 | frontend and workspace tooling |
| Rust 1.88+ (stable) | Tauri backend; on Windows the MSVC toolchain |
| Tauri 2 prerequisites | WebView2 on Windows (bundled bootstrapper), `webkit2gtk` on Linux, Xcode CLT on macOS |
| Claude Code (`claude` CLI) 2.0.73+ | signed in with `claude auth login` (Pro/Max/Team/Enterprise) |
| Google Chrome | with the **Claude in Chrome** extension (1.0.36+) installed and enabled |
| MongoDB Atlas cluster | optional; the app works local-only until configured |

## Quick start

```bash
pnpm install
pnpm tauri dev          # builds the Rust backend and opens the app
```

On first launch the setup wizard checks Claude CLI, Claude authentication, Chrome + Claude in Chrome, MongoDB Atlas, your master resume and your profile.

Build installers:

```bash
pnpm tauri build        # produces platform installers under target/release/bundle
```

Run everything that CI would run:

```bash
pnpm typecheck && pnpm lint && pnpm test      # TypeScript
cargo test --workspace                        # Rust unit + mock end-to-end tests
cargo clippy --workspace --all-targets -- -D warnings
```

## Repository layout

```
job-hunter/
├── apps/desktop/            Tauri desktop app
│   ├── src/                 React UI (Vite, Chakra UI, TanStack Query/Router)
│   └── src-tauri/           Tauri shell: commands + event forwarding
├── apps/mobile/             Flutter companion app (review-only, no AI) -- see docs/mobile.md
├── apps/backend/            Self-hosted Node/Fastify backend (built, not yet wired in) --
│                            owns MongoDB Atlas data directly -- see docs/backend.md
├── crates/job-hunter-core/  Rust core: domain, store + Atlas sync, Claude CLI runner,
│                            Chrome detection, documents, agent state machine, orchestrator,
│                            remote/ (dispatches phone requests to the same orchestrator calls)
├── crates/job-hunter-relay/ Self-hosted relay: accounts, paired devices, presence, request
│                            routing between the desktop and mobile app -- see docs/relay.md
├── packages/
│   ├── types/               Shared TS types (mirror of the Rust domain)
│   ├── shared/              Filters, status helpers, formatting (tested)
│   ├── agent-protocol/      Typed command + event names between UI and Rust
│   ├── ui/                  Small shared UI components
│   └── config/              tsconfig / eslint base
├── agent/
│   ├── system/job-hunter.md Claude system instructions (embedded at compile time)
│   ├── skills/*/SKILL.md    Modular skills (discovery, analysis, resume, chrome-application, …)
│   ├── workflows/           Job hunt + application workflow descriptions
│   ├── schemas/             JSON schemas for Claude structured output
│   └── fixtures/            Deterministic fixture jobs and candidate (mock mode + tests)
├── docs/                    Architecture, setup, Claude Code, Chrome, MongoDB, agent, security, …
├── scripts/                 check-all scripts
└── storage/                 Local placeholders (real user data lives in the app data dir)
```

## Documentation

- [docs/setup.md](docs/setup.md) — install everything and run the first job hunt
- [docs/architecture.md](docs/architecture.md) — how the pieces fit together
- [docs/claude-code.md](docs/claude-code.md) — how Claude Code is invoked (no API key)
- [docs/chrome.md](docs/chrome.md) — Claude in Chrome integration and limits
- [docs/mongodb.md](docs/mongodb.md) — Atlas setup and offline behaviour
- [docs/agent.md](docs/agent.md) — state machine, events, commands
- [docs/workflows.md](docs/workflows.md) — job hunt, approval, application, manual fallback
- [docs/mobile.md](docs/mobile.md) — the Flutter companion app: what it can/can't do, building it
- [docs/relay.md](docs/relay.md) — self-hosting the relay the mobile app connects through today
- [docs/backend.md](docs/backend.md) — the Node backend that will replace the relay's data path: what's built, what's still ahead
- [docs/mobile-protocol.md](docs/mobile-protocol.md) — the wire protocol between desktop, relay and phone
- [docs/security.md](docs/security.md) — what is never stored or logged
- [docs/development.md](docs/development.md) — commands, mock mode, tests
- [docs/troubleshooting.md](docs/troubleshooting.md)

## Guarantees the agent keeps

- Never fabricates skills, experience, employers, dates, degrees or certifications.
- Never submits an application without your explicit approval; automatic submission is not a setting.
- Never bypasses CAPTCHA or anti-bot protection; it stops and asks you.
- Never marks an application as submitted without visible evidence.
- Never stores Claude tokens, browser cookies, passwords or the MongoDB password in plain text.

## Status and known limitations

See the end of [docs/troubleshooting.md](docs/troubleshooting.md#known-limitations). In short: job sites change often and discovery quality depends on your Claude model and login state; V1 targets LinkedIn, Naukri, Indeed and Wellfound through browser interaction; applicant tracking systems with multi-step assessments are handed over to you.

License: MIT.
