# Development

## Commands

```bash
pnpm install                 # workspace dependencies
pnpm tauri dev               # run the desktop app (Vite + cargo build)
pnpm tauri build             # installers under target/release/bundle
pnpm typecheck               # tsc for every package
pnpm lint                    # eslint (desktop)
pnpm test                    # vitest (shared helpers + UI)
cargo test --workspace       # Rust unit tests + mock end-to-end pipeline
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
scripts/check-all.sh         # everything above (bash); scripts/check-all.ps1 on Windows
```

## Mock mode

Set `MOCK_MODE=true` (environment or `.env`) or toggle **Settings → Agent → Mock mode**. The agent then uses `MockClaudeRunner`:

- discovery returns `agent/fixtures/discovered-jobs.json` (3 LinkedIn + 2 Naukri jobs, one duplicated across platforms);
- analysis is a deterministic skill-overlap heuristic;
- resume generation builds the document from the candidate profile;
- validation flags skills that are not in the profile as unsupported claims;
- apply never submits (`MANUAL_ACTION_REQUIRED`), or simulates `HUMAN_INPUT_REQUIRED` / `SUBMITTED` when the `simulate` argument of `apply_application` is set (used by tests).

Secrets use an in-memory store in mock mode so tests never touch the OS keychain.

Use the fixture candidate for a quick demo: `agent/fixtures/candidate.json`.

## Tests

| where | what |
| --- | --- |
| `crates/job-hunter-core/src/**` unit tests | state machine, application transitions, dedup, settings, local store, secrets redaction, logging, stream parser, CLI shim, mock runner, document rendering |
| `crates/job-hunter-core/tests/mock_pipeline.rs` | full hunt → review → approve → manual fallback; human-input resume; stop; rejected jobs never applied |
| `packages/shared/src/index.test.ts` | job filters, application actions, profile validation |
| `apps/desktop/src/__tests__` | UI components, approval guards |

Real job submissions are never performed by automated tests.

### Opt-in smoke tests against the real Claude CLI

`crates/job-hunter-core/tests/real_claude_smoke.rs` is `#[ignore]`d so normal runs never spend your Claude quota:

```bash
# analysis + resume generation, no browser (about 3 minutes, a few cents)
cargo test -p job-hunter-core --test real_claude_smoke -- --ignored --nocapture

# additionally: read-only discovery through Claude in Chrome (Chrome must be running)
JOB_HUNTER_REAL_CHROME=1 JOB_HUNTER_SMOKE_SOURCE=Naukri \n  cargo test -p job-hunter-core --test real_claude_smoke real_chrome_discovery -- --ignored --nocapture
```

Set `JOB_HUNTER_SMOKE_DIR=<dir>` to keep the temporary data directory (prompts, transcripts, generated PDFs) for inspection. These tests use the fixture candidate in an isolated data directory and never touch your real Job Hunter data; discovery never creates applications.

## Environment variables

See `.env.example`. `JOB_HUNTER_DATA_DIR` overrides the data directory; `JOB_HUNTER_LOG` sets the tracing filter (e.g. `debug`).

## Adding a job source

1. Add the platform to `default_sources()` in `crates/job-hunter-core/src/settings.rs`.
2. Add a search recipe to `agent/skills/job-discovery/SKILL.md`.
3. Optionally add fixture jobs under that platform key in `agent/fixtures/discovered-jobs.json`.

## Adding a Claude task

1. Write the skill in `agent/skills/<name>/SKILL.md` and a schema in `agent/schemas/`.
2. Embed both in `prompts.rs` and add a prompt builder.
3. Implement the step in `agent/steps.rs`; give the request a `label` and, for mock mode, a `mock_context` plus a branch in `claude/mock.rs`.

## Code style

Rust: `rustfmt` (max width 100 in `rustfmt.toml`; long builder chains are fine) and clippy clean. TypeScript: strict mode, `consistent-type-imports`, no unused symbols.
