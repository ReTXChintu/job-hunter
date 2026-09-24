import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { FastifyInstance } from "fastify";
import { buildTestApp } from "./testApp.js";

describe("pairing routes", () => {
  let app: FastifyInstance;

  beforeEach(async () => {
    ({ app } = await buildTestApp());
  });
  afterEach(async () => {
    await app.close();
  });

  it("mints a code from a logged-in account and redeems it with no credentials", async () => {
    const register = await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email: "pair@example.com", password: "longenoughpassword" } });
    const { accessToken, userId } = register.json();

    const create = await app.inject({ method: "POST", url: "/v1/pairing/create", headers: { authorization: `Bearer ${accessToken}` }, payload: {} });
    expect(create.statusCode).toBe(200);
    const { code, expiresAt } = create.json();
    expect(code).toHaveLength(6);
    expect(new Date(expiresAt).getTime()).toBeGreaterThan(Date.now());

    const redeem = await app.inject({ method: "POST", url: "/v1/pairing/redeem", payload: { code, name: "My Phone", platform: "android" } });
    expect(redeem.statusCode).toBe(200);
    const redeemed = redeem.json();
    expect(redeemed.userId).toBe(userId);
    expect(redeemed.deviceToken).toBeTruthy();

    // Single-use: redeeming the same code again fails.
    const again = await app.inject({ method: "POST", url: "/v1/pairing/redeem", payload: { code, name: "My Phone", platform: "android" } });
    expect(again.statusCode).toBe(400);
    expect(again.json().error.code).toBe("INVALID_PAIRING_CODE");
  });

  it("accepts lowercase input and rejects an unknown code", async () => {
    const register = await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email: "pair2@example.com", password: "longenoughpassword" } });
    const { accessToken } = register.json();
    const create = await app.inject({ method: "POST", url: "/v1/pairing/create", headers: { authorization: `Bearer ${accessToken}` }, payload: {} });
    const { code } = create.json();

    const redeem = await app.inject({ method: "POST", url: "/v1/pairing/redeem", payload: { code: code.toLowerCase(), name: "Phone" } });
    expect(redeem.statusCode).toBe(200);

    const bogus = await app.inject({ method: "POST", url: "/v1/pairing/redeem", payload: { code: "ZZZZZZ", name: "Phone" } });
    expect(bogus.statusCode).toBe(400);
  });

  it("a device token alone can also mint a pairing code", async () => {
    const register = await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email: "pair3@example.com", password: "longenoughpassword" } });
    const { accessToken } = register.json();
    const deviceRes = await app.inject({
      method: "POST",
      url: "/v1/devices/register",
      headers: { authorization: `Bearer ${accessToken}` },
      payload: { name: "Desktop", kind: "desktop" },
    });
    const { deviceToken } = deviceRes.json();

    const create = await app.inject({ method: "POST", url: "/v1/pairing/create", headers: { authorization: `Bearer ${deviceToken}` }, payload: {} });
    expect(create.statusCode).toBe(200);
  });
});
