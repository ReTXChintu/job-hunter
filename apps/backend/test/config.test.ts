import { existsSync } from "node:fs";
import path from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { loadConfig } from "../src/config.js";
import { REPO_ROOT, ROOT_ENV_FILE } from "../src/paths.js";

const KEYS = ["JOB_HUNTER_BACKEND_MONGODB_URI", "JOB_HUNTER_BACKEND_WEB_DIR", "JOB_HUNTER_BACKEND_DOWNLOADS_DIR"] as const;
const saved = Object.fromEntries(KEYS.map((k) => [k, process.env[k]]));

afterEach(() => {
  for (const k of KEYS) {
    if (saved[k] === undefined) delete process.env[k];
    else process.env[k] = saved[k];
  }
});

describe("single root .env", () => {
  it("locates the repository root, where the one .env lives", () => {
    expect(existsSync(path.join(REPO_ROOT, "pnpm-workspace.yaml"))).toBe(true);
    expect(ROOT_ENV_FILE).toBe(path.join(REPO_ROOT, ".env"));
  });

  it("resolves relative paths against the repository root, not the working directory", () => {
    process.env.JOB_HUNTER_BACKEND_MONGODB_URI = "mongodb://unused";
    delete process.env.JOB_HUNTER_BACKEND_WEB_DIR;
    delete process.env.JOB_HUNTER_BACKEND_DOWNLOADS_DIR;
    let c = loadConfig();
    expect(c.webDir).toBe(path.join(REPO_ROOT, "apps", "web", "dist"));
    expect(c.downloadsDir).toBe(path.join(REPO_ROOT, "apps", "backend", "downloads"));

    process.env.JOB_HUNTER_BACKEND_DOWNLOADS_DIR = "srv/builds";
    process.env.JOB_HUNTER_BACKEND_WEB_DIR = path.join(REPO_ROOT, "elsewhere");
    c = loadConfig();
    expect(c.downloadsDir).toBe(path.join(REPO_ROOT, "srv", "builds"));
    expect(c.webDir).toBe(path.join(REPO_ROOT, "elsewhere"));
  });
});
