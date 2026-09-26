import { describe, expect, it } from "vitest";

import { PRESENCE_WINDOW_MS, recentlySeen } from "../src/presence.js";
import { buildTestApp } from "./testApp.js";

async function signUpWithDesktop(app: Awaited<ReturnType<typeof buildTestApp>>["app"]) {
  const reg = await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email: "presence@example.com", password: "longenoughpassword" } });
  const { accessToken } = reg.json();
  const dev = await app.inject({
    method: "POST",
    url: "/v1/devices/register",
    headers: { authorization: `Bearer ${accessToken}` },
    payload: { name: "Desktop", kind: "desktop" },
  });
  return { accessToken: accessToken as string, deviceId: dev.json().deviceId as string, deviceToken: dev.json().deviceToken as string };
}

async function desktopOnline(app: Awaited<ReturnType<typeof buildTestApp>>["app"], accessToken: string) {
  const res = await app.inject({ method: "GET", url: "/v1/devices", headers: { authorization: `Bearer ${accessToken}` } });
  return (res.json().devices as { kind: string; online: boolean }[]).find((d) => d.kind === "desktop")?.online;
}

describe("desktop presence without a WebSocket", () => {
  it("counts a desktop as online once it makes a request with its device token", async () => {
    const { app } = await buildTestApp();
    const { accessToken, deviceToken } = await signUpWithDesktop(app);
    // Registered, but hasn't talked to the server as a device yet.
    expect(await desktopOnline(app, accessToken)).toBe(false);

    // A sync request (or the idle heartbeat) with the device token.
    await app.inject({ method: "GET", url: "/v1/data/jobs", headers: { authorization: `Bearer ${deviceToken}` } });
    await new Promise((r) => setTimeout(r, 10)); // the "seen" write is fire-and-forget
    expect(await desktopOnline(app, accessToken)).toBe(true);
  });

  it("goes offline once not seen for longer than the presence window", async () => {
    const { app, db } = await buildTestApp();
    const { accessToken, deviceId } = await signUpWithDesktop(app);
    await db.touchDeviceLastSeen(deviceId, new Date(Date.now() - PRESENCE_WINDOW_MS - 1_000));
    expect(await desktopOnline(app, accessToken)).toBe(false);
    await db.touchDeviceLastSeen(deviceId, new Date(Date.now() - 30_000));
    expect(await desktopOnline(app, accessToken)).toBe(true);
  });

  it("recentlySeen handles missing and garbage timestamps", () => {
    expect(recentlySeen(null)).toBe(false);
    expect(recentlySeen("not a date")).toBe(false);
    expect(recentlySeen(new Date().toISOString())).toBe(true);
  });
});
