import { mkdtemp, rm, utimes, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { buildTestApp } from "./testApp.js";

describe("static serving: APK downloads and the web app", () => {
  let dir: string;

  beforeEach(async () => {
    dir = await mkdtemp(path.join(tmpdir(), "jh-static-"));
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("reports no Android build and 404s the download when none has been uploaded", async () => {
    const { app } = await buildTestApp({ downloadsDir: dir });
    const list = await app.inject({ method: "GET", url: "/v1/downloads" });
    expect(list.statusCode).toBe(200);
    expect(list.json()).toEqual({ android: null });
    const dl = await app.inject({ method: "GET", url: "/downloads/android" });
    expect(dl.statusCode).toBe(404);
    expect(dl.json().error.code).toBe("NOT_FOUND");
  });

  it("serves the newest APK as an attachment, ignoring other files", async () => {
    await writeFile(path.join(dir, "job-hunter-old.apk"), "old-apk");
    await writeFile(path.join(dir, "job-hunter-new.apk"), "new-apk-bytes");
    await writeFile(path.join(dir, "notes.txt"), "not an apk");
    const old = new Date("2026-01-01T00:00:00Z");
    await utimes(path.join(dir, "job-hunter-old.apk"), old, old);

    const { app } = await buildTestApp({ downloadsDir: dir });
    const list = await app.inject({ method: "GET", url: "/v1/downloads" });
    expect(list.json().android).toMatchObject({ fileName: "job-hunter-new.apk", sizeBytes: 13, url: "/downloads/android" });

    const dl = await app.inject({ method: "GET", url: "/downloads/android" });
    expect(dl.statusCode).toBe(200);
    expect(dl.headers["content-type"]).toBe("application/vnd.android.package-archive");
    expect(dl.headers["content-disposition"]).toBe('attachment; filename="job-hunter-new.apk"');
    expect(dl.body).toBe("new-apk-bytes");
  });

  it("serves the built web app at / without shadowing the API", async () => {
    await writeFile(path.join(dir, "index.html"), "<!doctype html><title>Job Hunter</title>");
    const { app } = await buildTestApp({ webDir: dir });
    const index = await app.inject({ method: "GET", url: "/" });
    expect(index.statusCode).toBe(200);
    expect(index.body).toContain("<title>Job Hunter</title>");
    const health = await app.inject({ method: "GET", url: "/healthz" });
    expect(health.body).toBe("ok");
    const unauthorized = await app.inject({ method: "GET", url: "/v1/applications" });
    expect(unauthorized.statusCode).toBe(401);
  });

  it("does not serve anything at / when the web app hasn't been built", async () => {
    const { app } = await buildTestApp();
    const index = await app.inject({ method: "GET", url: "/" });
    expect(index.statusCode).toBe(404);
  });
});
