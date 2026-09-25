/// <reference types="vitest/config" />
import { fileURLToPath } from "node:url";

import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";

// The repo's single .env lives at its root.
const repoRoot = fileURLToPath(new URL("../..", import.meta.url));

// In production the backend serves this app from the same origin as its
// API (see apps/backend/src/routes/static.ts), so every request is a
// relative path. In development, Vite proxies those paths to the backend
// the root .env points at.
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, repoRoot, "JOB_HUNTER_");
  const backend = env.JOB_HUNTER_BACKEND_URL || `http://127.0.0.1:${env.JOB_HUNTER_BACKEND_PORT || 8788}`;
  return {
    plugins: [react()],
    server: {
      port: 5173,
      proxy: {
        "/v1": backend,
        "/downloads": backend,
        "/healthz": backend,
      },
    },
    build: {
      // One ~160 KB gzipped bundle is fine for a small, LAN-hosted tool.
      chunkSizeWarningLimit: 800,
    },
    test: {
      environment: "node",
    },
  };
});
