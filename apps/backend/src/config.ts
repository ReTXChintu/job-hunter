/**
 * Backend configuration, loaded from environment variables. Mirrors
 * `crates/job-hunter-relay/src/config.rs` in shape (same variable names
 * where the concept carries over) so the deployment story stays familiar.
 * See `.env.example` at the repository root: the whole repo shares one `.env`.
 */
import path from "node:path";

import { REPO_ROOT } from "./paths.js";

export interface BackendConfig {
  port: number;
  mongoUri: string;
  mongoDatabase: string;
  jwtSecret: string;
  accessTokenTtlSecs: number;
  refreshTokenTtlDays: number;
  pairingCodeTtlSecs: number;
  corsOrigins: string[];
  /** Built web app (`apps/web/dist`) served at `/`. Skipped if missing. */
  webDir: string;
  /** Directory holding downloadable app builds (the Android APK). */
  downloadsDir: string;
}

function envInt(key: string, fallback: number): number {
  const raw = process.env[key];
  if (!raw) return fallback;
  const n = Number.parseInt(raw, 10);
  return Number.isFinite(n) ? n : fallback;
}

export function loadConfig(): BackendConfig {
  const mongoUri = process.env.JOB_HUNTER_BACKEND_MONGODB_URI ?? "";
  if (!mongoUri) {
    throw new Error(
      "JOB_HUNTER_BACKEND_MONGODB_URI must be set (the same Atlas connection string the desktop app already uses).",
    );
  }
  const jwtSecret = process.env.JOB_HUNTER_BACKEND_JWT_SECRET ?? "";
  if (!jwtSecret) {
    if (process.env.NODE_ENV === "production") {
      throw new Error("JOB_HUNTER_BACKEND_JWT_SECRET must be set to a long random value in production.");
    }
    console.warn("JOB_HUNTER_BACKEND_JWT_SECRET not set; using an insecure development default. Set it before deploying.");
  }
  return {
    port: envInt("JOB_HUNTER_BACKEND_PORT", 8788),
    mongoUri,
    mongoDatabase: process.env.JOB_HUNTER_BACKEND_MONGODB_DATABASE ?? "job_hunter",
    jwtSecret: jwtSecret || "dev-only-insecure-secret-change-me",
    accessTokenTtlSecs: envInt("JOB_HUNTER_BACKEND_ACCESS_TTL_SECS", 15 * 60),
    refreshTokenTtlDays: envInt("JOB_HUNTER_BACKEND_REFRESH_TTL_DAYS", 30),
    pairingCodeTtlSecs: envInt("JOB_HUNTER_BACKEND_PAIRING_TTL_SECS", 5 * 60),
    corsOrigins: (process.env.JOB_HUNTER_BACKEND_CORS_ORIGINS ?? "")
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean),
    // Relative paths are relative to the repository root, like the .env itself.
    webDir: path.resolve(REPO_ROOT, process.env.JOB_HUNTER_BACKEND_WEB_DIR || "apps/web/dist"),
    downloadsDir: path.resolve(REPO_ROOT, process.env.JOB_HUNTER_BACKEND_DOWNLOADS_DIR || "apps/backend/downloads"),
  };
}
