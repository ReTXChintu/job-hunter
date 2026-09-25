import { mkdtemp, rm, utimes, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { parseBuildName } from "../src/routes/static.js";
import { buildTestApp } from "./testApp.js";

describe("parseBuildName", () => {
  it("reads version and build number from the names CI produces", () => {
    expect(parseBuildName("job-hunter-0.2.0-15.apk")).toEqual({ version: "0.2.0", build: 15 });
    expect(parseBuildName("job-hunter-1.10.3.apk")).toEqual({ version: "1.10.3", build: null });
    expect(parseBuildName("job-hunter-0.3.0-rc.1-42.apk")).toEqual({ version: "0.3.0-rc.1", build: 42 });
    expect(parseBuildName("Job Hunter_0.2.0_x64-setup.exe")).toEqual({ version: "0.2.0", build: null });
    expect(parseBuildName("app-release.apk")).toEqual({ version: null, build: null });
  });
});

describe("static serving: downloads, updates and the web app", () => {
  let dir: string;

  beforeEach(async () => {
    dir = await mkdtemp(path.join(tmpdir(), "jh-static-"));
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("reports no builds and 404s downloads and the update manifest when none are uploaded", async () => {
    const { app } = await buildTestApp({ downloadsDir: dir });
    const list = await app.inject({ method: "GET", url: "/v1/downloads" });
    expect(list.statusCode).toBe(200);
    expect(list.json()).toEqual({ android: null, windows: null });
    for (const url of ["/downloads/android", "/downloads/windows", "/updates/windows/latest.json"]) {
      const res = await app.inject({ method: "GET", url });
      expect(res.statusCode, url).toBe(404);
      expect(res.json().error.code).toBe("NOT_FOUND");
    }
  });

  it("serves the newest APK as an attachment with its version, ignoring other files", async () => {
    await writeFile(path.join(dir, "job-hunter-0.1.0-3.apk"), "old-apk");
    await writeFile(path.join(dir, "job-hunter-0.2.0-7.apk"), "new-apk-bytes");
    await writeFile(path.join(dir, "notes.txt"), "not an apk");
    const old = new Date("2026-01-01T00:00:00Z");
    await utimes(path.join(dir, "job-hunter-0.1.0-3.apk"), old, old);

    const { app } = await buildTestApp({ downloadsDir: dir });
    const list = await app.inject({ method: "GET", url: "/v1/downloads" });
    expect(list.json().android).toMatchObject({ fileName: "job-hunter-0.2.0-7.apk", version: "0.2.0", build: 7, sizeBytes: 13, url: "/downloads/android" });

    const dl = await app.inject({ method: "GET", url: "/downloads/android" });
    expect(dl.statusCode).toBe(200);
    expect(dl.headers["content-type"]).toBe("application/vnd.android.package-archive");
    expect(dl.headers["content-disposition"]).toBe('attachment; filename="job-hunter-0.2.0-7.apk"');
    expect(dl.body).toBe("new-apk-bytes");
  });

  it("serves the Windows installer and a Tauri update manifest when it is signed", async () => {
    const exe = "Job Hunter_0.2.0_x64-setup.exe";
    await writeFile(path.join(dir, exe), "installer-bytes");

    const { app } = await buildTestApp({ downloadsDir: dir });
    let list = await app.inject({ method: "GET", url: "/v1/downloads" });
    expect(list.json().windows).toMatchObject({ fileName: exe, version: "0.2.0", signed: false, url: "/downloads/windows" });
    // Unsigned: downloadable, but no in-app update manifest.
    expect((await app.inject({ method: "GET", url: "/updates/windows/latest.json" })).statusCode).toBe(404);

    await writeFile(path.join(dir, `${exe}.sig`), "dW50cnVzdGVkIGNvbW1lbnQ=\n");
    list = await app.inject({ method: "GET", url: "/v1/downloads" });
    expect(list.json().windows.signed).toBe(true);

    const manifest = await app.inject({ method: "GET", url: "/updates/windows/latest.json", headers: { host: "10.0.0.5:8788" } });
    expect(manifest.statusCode).toBe(200);
    expect(manifest.json()).toMatchObject({
      version: "0.2.0",
      platforms: { "windows-x86_64": { signature: "dW50cnVzdGVkIGNvbW1lbnQ=", url: "http://10.0.0.5:8788/downloads/windows" } },
    });

    const dl = await app.inject({ method: "GET", url: "/downloads/windows" });
    expect(dl.statusCode).toBe(200);
    expect(dl.headers["content-disposition"]).toBe(`attachment; filename="${exe}"`);
    expect(dl.body).toBe("installer-bytes");
  });

  it("serves the built web app at / without shadowing the API", async () => {
    await writeFile(path.join(dir, "index.html"), "<!doctype html><title>Job Hunter</title>");
    const { app } = await buildTestApp({ webDir: dir });
    const index = await app.inject({ method: "GET", url: "/" });
    expect(index.statusCode).toBe(200);
    expect(index.body).toContain("<title>Job Hunter</title>");
    expect((await app.inject({ method: "GET", url: "/healthz" })).body).toBe("ok");
    expect((await app.inject({ method: "GET", url: "/v1/applications" })).statusCode).toBe(401);
  });

  it("does not serve anything at / when the web app hasn't been built", async () => {
    const { app } = await buildTestApp();
    expect((await app.inject({ method: "GET", url: "/" })).statusCode).toBe(404);
  });
});
