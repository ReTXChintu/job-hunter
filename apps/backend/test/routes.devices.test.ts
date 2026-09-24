import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { FastifyInstance } from "fastify";
import { buildTestApp } from "./testApp.js";

async function registerAndLogin(app: FastifyInstance) {
  const res = await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email: "dev@example.com", password: "longenoughpassword" } });
  return res.json() as { userId: string; accessToken: string; refreshToken: string };
}

describe("device routes", () => {
  let app: FastifyInstance;

  beforeEach(async () => {
    ({ app } = await buildTestApp());
  });
  afterEach(async () => {
    await app.close();
  });

  it("registers a device with an access token, then reaches devices with the device token alone", async () => {
    const session = await registerAndLogin(app);

    const register = await app.inject({
      method: "POST",
      url: "/v1/devices/register",
      headers: { authorization: `Bearer ${session.accessToken}` },
      payload: { name: "My Desktop", kind: "desktop", platform: "windows" },
    });
    expect(register.statusCode).toBe(200);
    const { deviceId, deviceToken } = register.json();
    expect(deviceId).toBeTruthy();
    expect(deviceToken).toBeTruthy();

    // The access token is never needed again -- the device token alone
    // authorizes /devices and /pairing/create (see docs/mobile-protocol.md).
    const list = await app.inject({ method: "GET", url: "/v1/devices", headers: { authorization: `Bearer ${deviceToken}` } });
    expect(list.statusCode).toBe(200);
    expect(list.json().devices).toHaveLength(1);
    expect(list.json().devices[0].id).toBe(deviceId);
    expect(list.json().devices[0].name).toBe("My Desktop");
  });

  it("rejects an unknown device kind", async () => {
    const session = await registerAndLogin(app);
    const res = await app.inject({
      method: "POST",
      url: "/v1/devices/register",
      headers: { authorization: `Bearer ${session.accessToken}` },
      payload: { name: "Thing", kind: "toaster" },
    });
    expect(res.statusCode).toBe(400);
    expect(res.json().error.code).toBe("VALIDATION");
  });

  it("returns 401 with no Authorization header", async () => {
    const res = await app.inject({ method: "GET", url: "/v1/devices" });
    expect(res.statusCode).toBe(401);
    expect(res.json().error.code).toBe("UNAUTHORIZED");
  });

  it("deletes a device and reports 404 for an unknown one", async () => {
    const session = await registerAndLogin(app);
    const register = await app.inject({
      method: "POST",
      url: "/v1/devices/register",
      headers: { authorization: `Bearer ${session.accessToken}` },
      payload: { name: "Phone", kind: "mobile" },
    });
    const { deviceId } = register.json();

    const del = await app.inject({ method: "DELETE", url: `/v1/devices/${deviceId}`, headers: { authorization: `Bearer ${session.accessToken}` } });
    expect(del.statusCode).toBe(200);

    const again = await app.inject({ method: "DELETE", url: `/v1/devices/${deviceId}`, headers: { authorization: `Bearer ${session.accessToken}` } });
    expect(again.statusCode).toBe(404);
    expect(again.json().error.code).toBe("DEVICE_NOT_FOUND");
  });
});
