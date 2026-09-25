/**
 * Static serving for a self-hosted, domain-less deployment (reached as
 * `http://<ip>:<port>`):
 *
 * - `GET /` and friends: the read-only web app (`apps/web/dist`), served
 *   from the same origin as the API so it needs no CORS or configured URL.
 *   Skipped when the directory doesn't exist (e.g. in development, where
 *   the web app runs on Vite's own dev server instead).
 * - `GET /v1/downloads`: what's available to download, as JSON.
 * - `GET /downloads/android`: the newest `.apk` in the downloads directory.
 *
 * Nothing here requires auth: the web app's shell and the APK are not
 * secret, and every piece of data behind them still is.
 */
import { createReadStream } from "node:fs";
import { readdir, stat } from "node:fs/promises";
import path from "node:path";

import fastifyStatic from "@fastify/static";
import type { FastifyInstance } from "fastify";

import { Errors } from "../errors.js";
import type { AppState } from "../state.js";

export interface DownloadInfo {
  fileName: string;
  sizeBytes: number;
  updatedAt: string;
  url: string;
}

/** Newest file in `dir` with the given extension, or null if there is none. */
export async function newestFile(dir: string, extension: string): Promise<{ fullPath: string; info: DownloadInfo } | null> {
  let names: string[];
  try {
    names = await readdir(dir);
  } catch {
    return null;
  }
  let best: { fullPath: string; name: string; size: number; mtime: Date } | null = null;
  for (const name of names) {
    if (!name.toLowerCase().endsWith(extension)) continue;
    const fullPath = path.join(dir, name);
    const s = await stat(fullPath).catch(() => null);
    if (!s?.isFile()) continue;
    if (!best || s.mtime > best.mtime) best = { fullPath, name, size: s.size, mtime: s.mtime };
  }
  if (!best) return null;
  return {
    fullPath: best.fullPath,
    info: { fileName: best.name, sizeBytes: best.size, updatedAt: best.mtime.toISOString(), url: "/downloads/android" },
  };
}

async function dirHasIndex(dir: string): Promise<boolean> {
  const s = await stat(path.join(dir, "index.html")).catch(() => null);
  return !!s?.isFile();
}

export async function registerStaticRoutes(app: FastifyInstance, state: AppState): Promise<void> {
  const { downloadsDir, webDir } = state.config;

  app.get("/v1/downloads", async () => {
    const android = await newestFile(downloadsDir, ".apk");
    return { android: android?.info ?? null };
  });

  app.get("/downloads/android", async (_request, reply) => {
    const android = await newestFile(downloadsDir, ".apk");
    if (!android) throw Errors.notFound("No Android build has been uploaded to this server yet.");
    reply
      .header("Content-Type", "application/vnd.android.package-archive")
      .header("Content-Length", android.info.sizeBytes)
      .header("Content-Disposition", `attachment; filename="${android.info.fileName}"`);
    return reply.send(createReadStream(android.fullPath));
  });

  if (await dirHasIndex(webDir)) {
    await app.register(fastifyStatic, { root: webDir, prefix: "/" });
    app.log.info({ webDir }, "serving the web app");
  } else {
    app.log.info({ webDir }, "web app build not found; not serving it (run `pnpm server:build`)");
  }
}
