# Run every check CI would run (Windows / PowerShell).
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")
Write-Host "== TypeScript typecheck"; pnpm -r typecheck
Write-Host "== ESLint";              pnpm -r lint
Write-Host "== Vitest";              pnpm -r test
Write-Host "== cargo fmt";           cargo fmt --all -- --check
Write-Host "== cargo clippy";        cargo clippy --workspace --all-targets -- -D warnings
Write-Host "== cargo test";          cargo test --workspace
Write-Host "All checks passed."
