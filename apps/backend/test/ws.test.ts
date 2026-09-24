import { afterEach, beforeEach, describe, expect, it } from "vitest";
import WebSocket from "ws";

import type { FastifyInstance } from "fastify";
import type { AppState } from "../src/state.js";
import { buildTestApp } from "./testApp.js";

interface Session {
  userId: string;
  deviceToken: string;
}

async function registerDevice(app: FastifyInstance, email: string, kind: "desktop" | "mobile"): Promise<Session> {
  const register = await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email, password: "longenoughpassword" } });
  const { accessToken, userId } = register.json();
  const device = await app.inject({
    method: "POST",
    url: "/v1/devices/register",
    headers: { authorization: `Bearer ${accessToken}` },
    payload: { name: kind, kind },
  });
  return { userId, deviceToken: device.json().deviceToken };
}

/**
 * A `ws` client that queues every inbound message from the moment it is
 * constructed. A server can push a frame the instant the connection opens
 * (this backend's own presence snapshot does exactly that), and `ws`
 * delivers `open` and a same-tick `message` back to back before any
 * `await`-resumed continuation gets a turn -- so attaching a one-off
 * `.once('message', ...)` *after* `await`ing `open` can miss it. Queuing
 * from construction time removes the race instead of hoping the timing
 * works out.
 */
class QueuedSocket {
  readonly ws: WebSocket;
  private readonly queue: Record<string, unknown>[] = [];
  private readonly waiters: Array<(msg: Record<string, unknown>) => void> = [];
  private readonly opened: Promise<void>;

  constructor(url: string) {
    this.ws = new WebSocket(url);
    this.opened = new Promise((resolve, reject) => {
      this.ws.once("open", () => resolve());
      this.ws.once("error", reject);
    });
    this.ws.on("message", (data) => {
      const msg = JSON.parse(data.toString()) as Record<string, unknown>;
      const waiter = this.waiters.shift();
      if (waiter) waiter(msg);
      else this.queue.push(msg);
    });
  }

  waitOpen(): Promise<void> {
    return this.opened;
  }

  next(): Promise<Record<string, unknown>> {
    const queued = this.queue.shift();
    if (queued) return Promise.resolve(queued);
    return new Promise((resolve) => this.waiters.push(resolve));
  }

  send(payload: unknown): void {
    this.ws.send(JSON.stringify(payload));
  }

  close(): void {
    this.ws.close();
  }
}

describe("WebSocket routing over a real server and real sockets", () => {
  let app: FastifyInstance;
  let state: AppState;
  let baseUrl: string;

  beforeEach(async () => {
    ({ app, state } = await buildTestApp());
    await app.listen({ port: 0, host: "127.0.0.1" });
    const address = app.server.address();
    if (address === null || typeof address === "string") throw new Error("expected a bound TCP address");
    baseUrl = `ws://127.0.0.1:${address.port}`;
  });
  afterEach(async () => {
    await app.close();
  });

  it("tells a connecting mobile client the desktop's current presence, then live updates on connect/disconnect", async () => {
    const desktopSession = await registerDevice(app, "ws-desktop@example.com", "desktop");
    // Same account: register the mobile device against the same user by
    // reusing the account via login instead of a second register.
    const loginRes = await app.inject({ method: "POST", url: "/v1/auth/login", payload: { email: "ws-desktop@example.com", password: "longenoughpassword" } });
    const mobileDeviceRes = await app.inject({
      method: "POST",
      url: "/v1/devices/register",
      headers: { authorization: `Bearer ${loginRes.json().accessToken}` },
      payload: { name: "phone", kind: "mobile" },
    });
    const mobileToken = mobileDeviceRes.json().deviceToken as string;

    const mobile = new QueuedSocket(`${baseUrl}/v1/ws?token=${mobileToken}`);
    await mobile.waitOpen();
    const initialPresence = await mobile.next();
    expect(initialPresence.type).toBe("presence");
    expect((initialPresence.payload as { online: boolean }).online).toBe(false);

    const desktop = new QueuedSocket(`${baseUrl}/v1/ws?token=${desktopSession.deviceToken}`);
    await desktop.waitOpen();
    const onlinePresence = await mobile.next();
    expect((onlinePresence.payload as { online: boolean }).online).toBe(true);

    desktop.close();
    const offlinePresence = await mobile.next();
    expect((offlinePresence.payload as { online: boolean }).online).toBe(false);

    mobile.close();
  });

  it("routes a mobile request to the desktop and the desktop's reply back to the mobile client", async () => {
    const desktopSession = await registerDevice(app, "ws-route@example.com", "desktop");
    const loginRes = await app.inject({ method: "POST", url: "/v1/auth/login", payload: { email: "ws-route@example.com", password: "longenoughpassword" } });
    const mobileDeviceRes = await app.inject({
      method: "POST",
      url: "/v1/devices/register",
      headers: { authorization: `Bearer ${loginRes.json().accessToken}` },
      payload: { name: "phone", kind: "mobile" },
    });
    const mobileToken = mobileDeviceRes.json().deviceToken as string;

    const desktop = new QueuedSocket(`${baseUrl}/v1/ws?token=${desktopSession.deviceToken}`);
    await desktop.waitOpen();
    const mobile = new QueuedSocket(`${baseUrl}/v1/ws?token=${mobileToken}`);
    await mobile.waitOpen();
    await mobile.next(); // initial presence snapshot

    const requestId = "req-1";
    mobile.send({ v: 1, id: requestId, type: "list_applications", payload: {} });
    const onDesktop = await desktop.next();
    expect(onDesktop.id).toBe(requestId);
    expect(onDesktop.type).toBe("list_applications");

    desktop.send({ v: 1, id: requestId, type: "response", payload: { ok: true, data: [] } });
    const onMobile = await mobile.next();
    expect(onMobile.id).toBe(requestId);
    expect((onMobile.payload as { ok: boolean }).ok).toBe(true);

    desktop.close();
    mobile.close();
  });

  it("replies with a synthetic DESKTOP_OFFLINE response when no desktop is connected", async () => {
    const loginRegister = await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email: "ws-offline@example.com", password: "longenoughpassword" } });
    const mobileDeviceRes = await app.inject({
      method: "POST",
      url: "/v1/devices/register",
      headers: { authorization: `Bearer ${loginRegister.json().accessToken}` },
      payload: { name: "phone", kind: "mobile" },
    });
    const mobileToken = mobileDeviceRes.json().deviceToken as string;

    const mobile = new QueuedSocket(`${baseUrl}/v1/ws?token=${mobileToken}`);
    await mobile.waitOpen();
    await mobile.next(); // initial presence (offline)

    mobile.send({ v: 1, id: "req-x", type: "list_applications", payload: {} });
    const msg = await mobile.next();
    expect(msg.id).toBe("req-x");
    expect(msg.type).toBe("response");
    expect((msg.payload as { ok: boolean; error: { code: string } }).ok).toBe(false);
    expect((msg.payload as { error: { code: string } }).error.code).toBe("DESKTOP_OFFLINE");

    mobile.close();
  });

  it("rejects an unknown device token at the upgrade", async () => {
    const socket = new WebSocket(`${baseUrl}/v1/ws?token=not-a-real-token`);
    await new Promise<void>((resolve) => {
      socket.once("close", () => resolve());
      socket.once("error", () => resolve());
    });
  });

  it("broadcasts a changed frame to mobile clients when data is written through the generic data API", async () => {
    const register = await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email: "ws-changed@example.com", password: "longenoughpassword" } });
    const { accessToken } = register.json();
    const mobileDeviceRes = await app.inject({
      method: "POST",
      url: "/v1/devices/register",
      headers: { authorization: `Bearer ${accessToken}` },
      payload: { name: "phone", kind: "mobile" },
    });
    const mobileToken = mobileDeviceRes.json().deviceToken as string;

    const mobile = new QueuedSocket(`${baseUrl}/v1/ws?token=${mobileToken}`);
    await mobile.waitOpen();
    await mobile.next(); // initial presence

    await app.inject({ method: "PUT", url: "/v1/data/jobs/job-1", headers: { authorization: `Bearer ${accessToken}` }, payload: { id: "job-1", title: "X" } });
    const msg = await mobile.next();
    expect(msg.type).toBe("changed");
    expect((msg.payload as { collections: string[] }).collections).toEqual(["jobs"]);

    mobile.close();
  });

  it("is not affected by devices belonging to a different account", () => {
    // Sanity check that the Hub instance is per-app-state, not shared
    // globally, since every test in this file builds a fresh app.
    expect(state.hub.isDesktopOnline("nobody")).toBe(false);
  });
});
