#!/usr/bin/env node
/**
 * Sets one version across every app in the monorepo. release-it bumps the
 * root package.json and then runs this (see .release-it.json), so a single
 * `pnpm release` keeps all of these in step:
 *
 *   apps/*\/package.json                   "version"
 *   apps/desktop/src-tauri/tauri.conf.json "version"  (installer + in-app version)
 *   Cargo.toml [workspace.package]         version    (all Rust crates)
 *   Cargo.lock                             the workspace crates' entries
 *   apps/mobile/pubspec.yaml               version: X.Y.Z+N  (N incremented)
 *
 * Usage:
 *   node scripts/sync-versions.mjs 1.2.3     set everything to 1.2.3
 *   node scripts/sync-versions.mjs --check   fail if anything disagrees with the root package.json
 */
import { readdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const RUST_CRATES = ["job-hunter-core", "job-hunter-desktop", "job-hunter-relay"];

const read = (rel) => readFileSync(path.join(root, rel), "utf8");
const write = (rel, text) => writeFileSync(path.join(root, rel), text);

/** Replace exactly one match of `pattern`, or throw: never write a file silently unchanged. */
function replaceOnce(rel, pattern, replacement) {
  const text = read(rel);
  const matches = text.match(new RegExp(pattern.source, pattern.flags.includes("g") ? pattern.flags : pattern.flags + "g"));
  if (!matches || matches.length !== 1) throw new Error(`${rel}: expected exactly one match for ${pattern}, found ${matches?.length ?? 0}`);
  write(rel, text.replace(pattern, replacement));
}

function appPackageJsons() {
  return readdirSync(path.join(root, "apps"))
    .map((dir) => path.join("apps", dir, "package.json"))
    .filter((rel) => existsSync(path.join(root, rel)));
}

/** Every place a version lives, as [label, current version]. */
function currentVersions() {
  const out = [["package.json", JSON.parse(read("package.json")).version]];
  for (const rel of appPackageJsons()) out.push([rel, JSON.parse(read(rel)).version]);
  out.push(["apps/desktop/src-tauri/tauri.conf.json", JSON.parse(read("apps/desktop/src-tauri/tauri.conf.json")).version]);
  out.push(["Cargo.toml", read("Cargo.toml").match(/\[workspace\.package\][^[]*?\r?\nversion = "([^"]+)"/)?.[1]]);
  const lock = read("Cargo.lock");
  for (const name of RUST_CRATES) {
    out.push([`Cargo.lock (${name})`, lock.match(new RegExp(`name = "${name}"\\r?\nversion = "([^"]+)"`))?.[1]]);
  }
  out.push(["apps/mobile/pubspec.yaml", read("apps/mobile/pubspec.yaml").match(/^version: *([^+\s]+)/m)?.[1]]);
  return out;
}

function setVersion(version) {
  for (const rel of appPackageJsons()) replaceOnce(rel, /^(  "version": )"[^"]*"/m, `$1"${version}"`);
  replaceOnce("apps/desktop/src-tauri/tauri.conf.json", /^(  "version": )"[^"]*"/m, `$1"${version}"`);
  replaceOnce("Cargo.toml", /(\[workspace\.package\][^[]*?\r?\nversion = )"[^"]*"/, `$1"${version}"`);
  for (const name of RUST_CRATES) {
    replaceOnce("Cargo.lock", new RegExp(`(name = "${name}"\\r?\nversion = )"[^"]*"`), `$1"${version}"`);
  }
  // Keep the local build number increasing; CI builds override it with the run number.
  const build = Number(read("apps/mobile/pubspec.yaml").match(/^version: *[^+\s]+\+(\d+)/m)?.[1] ?? 0) + 1;
  replaceOnce("apps/mobile/pubspec.yaml", /^version: *[^\s]+/m, `version: ${version}+${build}`);
}

const arg = process.argv[2];
if (arg === "--check") {
  const versions = currentVersions();
  const expected = versions[0][1];
  const wrong = versions.filter(([, v]) => v !== expected);
  for (const [label, v] of versions) console.log(`${v === expected ? "ok " : "BAD"} ${label}: ${v}`);
  if (wrong.length) {
    console.error(`\n${wrong.length} file(s) disagree with package.json (${expected}). Run: node scripts/sync-versions.mjs ${expected}`);
    process.exit(1);
  }
} else if (arg && SEMVER.test(arg)) {
  setVersion(arg);
  console.log(`All apps set to ${arg}.`);
} else {
  console.error("Usage: node scripts/sync-versions.mjs <x.y.z> | --check");
  process.exit(2);
}
