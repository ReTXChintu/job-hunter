import { buildApp } from "../src/app.js";
import type { BackendConfig } from "../src/config.js";
import { createAppState } from "../src/state.js";
import { InMemoryDb } from "./inMemoryDb.js";

export function testConfig(overrides: Partial<BackendConfig> = {}): BackendConfig {
  return {
    port: 0,
    mongoUri: "unused-in-tests",
    mongoDatabase: "unused-in-tests",
    jwtSecret: "test-secret",
    accessTokenTtlSecs: 900,
    refreshTokenTtlDays: 30,
    pairingCodeTtlSecs: 300,
    corsOrigins: [],
    webDir: "/nonexistent/job-hunter-web",
    downloadsDir: "/nonexistent/job-hunter-downloads",
    ...overrides,
  };
}

export async function buildTestApp(overrides: Partial<BackendConfig> = {}) {
  const db = new InMemoryDb();
  const state = createAppState(testConfig(overrides), db);
  const app = await buildApp(state);
  return { app, state, db };
}
