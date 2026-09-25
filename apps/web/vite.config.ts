/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// In production the backend serves this app from the same origin as its
// API (see apps/backend/src/routes/static.ts), so every request is a
// relative path. In development, Vite proxies those paths to a backend
// running locally.
const backend = process.env.JOB_HUNTER_BACKEND_URL ?? "http://127.0.0.1:8788";

export default defineConfig({
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
});
