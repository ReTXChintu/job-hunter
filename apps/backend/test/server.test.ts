import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import path from "node:path";

import type { FastifyInstance } from "fastify";
import { afterEach, describe, expect, it } from "vitest";
import WebSocket from "ws";

import { planListeners, startServers } from "../src/server.js";
import { createAppState } from "../src/state.js";
import { InMemoryDb } from "./inMemoryDb.js";
import { testConfig } from "./testApp.js";

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const srv = createServer();
    srv.listen(0, "127.0.0.1", () => {
      const address = srv.address();
      srv.close(() => (typeof address === "object" && address ? resolve(address.port) : reject(new Error("no port"))));
    });
  });
}

describe("planListeners", () => {
  it("uses one port for everything when no separate web port is set", () => {
    expect(planListeners({ port: 50007, webPort: null })).toEqual([{ port: 50007, serveWeb: true, label: "api+web" }]);
    expect(planListeners({ port: 50007, webPort: 50007 })).toEqual([{ port: 50007, serveWeb: true, label: "api+web" }]);
  });

  it("splits the API and the web app across two ports", () => {
    expect(planListeners({ port: 50007, webPort: 50006 })).toEqual([
      { port: 50007, serveWeb: false, label: "api" },
      { port: 50006, serveWeb: true, label: "web" },
    ]);
  });
});

describe("startServers with a separate web port", () => {
  let apps: FastifyInstance[] = [];
  let webDir = "";

  afterEach(async () => {
    await Promise.all(apps.map((a) => a.close()));
    apps = [];
    if (webDir) await rm(webDir, { recursive: true, force: true });
  });

  it("serves the web app only on the web port, and both ports share one state", async () => {
    webDir = await mkdtemp(path.join(tmpdir(), "jh-web-"));
    await writeFile(path.join(webDir, "index.html"), "<!doctype html><title>Job Hunter</title>");
    const [apiPort, webPort] = [await freePort(), await freePort()];
    const state = createAppState(testConfig({ port: apiPort, webPort, webDir }), new InMemoryDb());
    apps = await startServers(state, "127.0.0.1");
    const api = `http://127.0.0.1:${apiPort}`;
    const web = `http://127.0.0.1:${webPort}`;

    expect((await fetch(`${api}/`)).status).toBe(404);
    expect(await (await fetch(`${web}/`)).text()).toContain("<title>Job Hunter</title>");
    expect(await (await fetch(`${api}/healthz`)).text()).toBe("ok");
    expect(await (await fetch(`${web}/healthz`)).text()).toBe("ok");

    // The web page's own API calls work on the web port (same origin)...
    const reg = await fetch(`${web}/v1/auth/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email: "ports@example.com", password: "longenoughpassword" }),
    });
    const { accessToken } = (await reg.json()) as { accessToken: string };
    const device = await fetch(`${api}/v1/devices/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Authorization: `Bearer ${accessToken}` },
      body: JSON.stringify({ name: "Desktop", kind: "desktop" }),
    });
    const { deviceToken } = (await device.json()) as { deviceToken: string };

    // ...and a desktop connected on the API port shows as online through the web port.
    const socket = new WebSocket(`ws://127.0.0.1:${apiPort}/v1/ws?token=${deviceToken}`);
    await new Promise<void>((resolve, reject) => {
      socket.once("open", () => resolve());
      socket.once("error", reject);
    });
    const devices = (await (await fetch(`${web}/v1/devices`, { headers: { Authorization: `Bearer ${accessToken}` } })).json()) as {
      devices: { kind: string; online: boolean }[];
    };
    expect(devices.devices.find((d) => d.kind === "desktop")?.online).toBe(true);
    socket.close();
  });
});
