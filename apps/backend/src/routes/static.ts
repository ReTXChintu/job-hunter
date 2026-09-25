/**
 * Static serving for a self-hosted, domain-less deployment (reached as
 * `http://<ip>:<port>`):
 *
 * - `GET /` and friends: the read-only web app (`apps/web/dist`), served
 *   from the same origin as the API so it needs no CORS or configured URL.
 *   Skipped when the directory doesn't exist (e.g. in development, where
 *   the web app runs on Vite's own dev server instead).
 * - `GET /v1/downloads`: the latest Android and Windows builds and their
 *   versions. The desktop and mobile apps' update checks read this.
 * - `GET /downloads/android`, `GET /downloads/windows`: the newest APK /
 *   NSIS installer in the downloads directory.
 * - `GET /updates/windows/latest.json`: the manifest Tauri's updater reads
 *   for in-app updates. Only served when the installer has its `.sig`
 *   (i.e. CI built it with the updater signing key).
 *
 * Nothing here requires auth: the web app's shell and the builds are not
 * secret, and every piece of data behind them still is.
 */
import { createReadStream } from "node:fs";
import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";

import fastifyStatic from "@fastify/static";
import type { FastifyInstance, FastifyReply } from "fastify";

import { Errors } from "../errors.js";
import type { AppState } from "../state.js";

export type Platform = "android" | "windows";

export interface DownloadInfo {
  fileName: string;
  /** Semver from the file name, e.g. "0.2.0"; null if it doesn't carry one. */
  version: string | null;
  /** Android build number (versionCode) from the file name, when present. */
  build: number | null;
  sizeBytes: number;
  updatedAt: string;
  url: string;
  /** Windows only: whether a Tauri updater signature sits next to it. */
  signed?: boolean;
}

const MATCHERS: Record<Platform, { test: (name: string) => boolean; mime: string }> = {
  android: { test: (n) => n.toLowerCase().endsWith(".apk"), mime: "application/vnd.android.package-archive" },
  // NSIS installers from `tauri build`: "Job Hunter_0.2.0_x64-setup.exe".
  windows: { test: (n) => n.toLowerCase().endsWith("-setup.exe"), mime: "application/vnd.microsoft.portable-executable" },
};

/**
 * Version and build number from a build's file name:
 * `job-hunter-0.2.0-15.apk` (CI's APK name) → 0.2.0, build 15;
 * `Job Hunter_0.2.0_x64-setup.exe` → 0.2.0.
 */
export function parseBuildName(fileName: string): { version: string | null; build: number | null } {
  // A prerelease tag must start with a letter ("-rc.1") so it can't be
  // confused with a build number ("-15").
  const m = fileName.match(/(\d+\.\d+\.\d+(?:-[A-Za-z][0-9A-Za-z.]*)?)(?:[-+](\d+))?(?=[._-])/);
  if (!m) return { version: null, build: null };
  return { version: m[1]!, build: m[2] ? Number(m[2]) : null };
}

/** Newest build for a platform in `dir`, or null if there is none. */
export async function newestBuild(dir: string, platform: Platform): Promise<{ fullPath: string; info: DownloadInfo } | null> {
  let names: string[];
  try {
    names = await readdir(dir);
  } catch {
    return null;
  }
  let best: { fullPath: string; name: string; size: number; mtime: Date } | null = null;
  for (const name of names) {
    if (!MATCHERS[platform].test(name)) continue;
    const fullPath = path.join(dir, name);
    const s = await stat(fullPath).catch(() => null);
    if (!s?.isFile()) continue;
    if (!best || s.mtime > best.mtime) best = { fullPath, name, size: s.size, mtime: s.mtime };
  }
  if (!best) return null;
  const info: DownloadInfo = {
    fileName: best.name,
    ...parseBuildName(best.name),
    sizeBytes: best.size,
    updatedAt: best.mtime.toISOString(),
    url: `/downloads/${platform}`,
  };
  if (platform === "windows") {
    info.signed = !!(await stat(`${best.fullPath}.sig`).catch(() => null))?.isFile();
  }
  return { fullPath: best.fullPath, info };
}

async function sendBuild(reply: FastifyReply, dir: string, platform: Platform) {
  const build = await newestBuild(dir, platform);
  if (!build) throw Errors.notFound(`No ${platform === "android" ? "Android" : "Windows"} build has been uploaded to this server yet.`);
  reply
    .header("Content-Type", MATCHERS[platform].mime)
    .header("Content-Length", build.info.sizeBytes)
    .header("Content-Disposition", `attachment; filename="${build.info.fileName}"`);
  return reply.send(createReadStream(build.fullPath));
}

async function dirHasIndex(dir: string): Promise<boolean> {
  const s = await stat(path.join(dir, "index.html")).catch(() => null);
  return !!s?.isFile();
}

export async function registerStaticRoutes(app: FastifyInstance, state: AppState): Promise<void> {
  const { downloadsDir, webDir } = state.config;

  app.get("/v1/downloads", async () => {
    const [android, windows] = await Promise.all([newestBuild(downloadsDir, "android"), newestBuild(downloadsDir, "windows")]);
    return { android: android?.info ?? null, windows: windows?.info ?? null };
  });

  app.get("/downloads/android", async (_request, reply) => sendBuild(reply, downloadsDir, "android"));
  app.get("/downloads/windows", async (_request, reply) => sendBuild(reply, downloadsDir, "windows"));

  // Tauri updater "static JSON" format. The URL must be absolute, so it's
  // built from the address the desktop used to reach us.
  app.get("/updates/windows/latest.json", async (request) => {
    const build = await newestBuild(downloadsDir, "windows");
    if (!build?.info.version || !build.info.signed) throw Errors.notFound("No signed Windows update is available.");
    const signature = (await readFile(`${build.fullPath}.sig`, "utf8")).trim();
    const origin = `${request.protocol}://${request.headers.host ?? request.hostname}`;
    return {
      version: build.info.version,
      notes: "",
      pub_date: build.info.updatedAt,
      platforms: {
        "windows-x86_64": { signature, url: `${origin}${build.info.url}` },
      },
    };
  });

  if (await dirHasIndex(webDir)) {
    await app.register(fastifyStatic, { root: webDir, prefix: "/" });
    app.log.info({ webDir }, "serving the web app");
  } else {
    app.log.info({ webDir }, "web app build not found; not serving it (run `pnpm server:build`)");
  }
}
