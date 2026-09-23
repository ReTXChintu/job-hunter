# Job Hunter – notes for Claude Code sessions in this repo

- Monorepo: pnpm workspace (`apps/desktop`, `packages/*`) + Cargo workspace (`crates/job-hunter-core`, `apps/desktop/src-tauri`).
- Run checks with `scripts/check-all.sh` (or `.ps1`). Rust tests include a mock end-to-end pipeline in `crates/job-hunter-core/tests/mock_pipeline.rs`.
- The agent's instructions live in `agent/` and are embedded at compile time by `crates/job-hunter-core/src/prompts.rs`; edit the markdown/schema files, then rebuild.
- TS types in `packages/types` must mirror the Rust domain structs (serde camelCase). Change both together.
- Never add an Anthropic API key or direct API calls: all AI runs through the user's `claude` CLI (`claude/runner.rs`).
- Approval gating is enforced in `domain/application.rs` (`can_transition_to`) and `agent/steps.rs::apply`; do not weaken it.
- Mock mode (`MOCK_MODE=true`) uses `claude/mock.rs` and `agent/fixtures`; keep it deterministic and never let it report a real submission.
