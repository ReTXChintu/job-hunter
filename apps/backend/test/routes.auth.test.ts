import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { FastifyInstance } from "fastify";
import { buildTestApp } from "./testApp.js";

describe("auth routes", () => {
  let app: FastifyInstance;

  beforeEach(async () => {
    ({ app } = await buildTestApp());
  });
  afterEach(async () => {
    await app.close();
  });

  it("registers, then logs in, then refreshes, then logs out", async () => {
    const register = await app.inject({
      method: "POST",
      url: "/v1/auth/register",
      payload: { email: "User@Example.com", password: "correct horse battery" },
    });
    expect(register.statusCode).toBe(200);
    const registerBody = register.json();
    expect(registerBody.userId).toBeTruthy();
    expect(registerBody.accessToken).toBeTruthy();
    expect(registerBody.refreshToken).toBeTruthy();

    const login = await app.inject({
      method: "POST",
      url: "/v1/auth/login",
      payload: { email: "user@example.com", password: "correct horse battery" },
    });
    expect(login.statusCode).toBe(200);
    expect(login.json().userId).toBe(registerBody.userId);

    const refresh = await app.inject({
      method: "POST",
      url: "/v1/auth/refresh",
      payload: { refreshToken: login.json().refreshToken },
    });
    expect(refresh.statusCode).toBe(200);
    const refreshBody = refresh.json();
    expect(refreshBody.accessToken).toBeTruthy();
    expect(refreshBody.refreshToken).not.toBe(login.json().refreshToken);

    // The old refresh token was rotated out and can't be reused.
    const reuseOld = await app.inject({
      method: "POST",
      url: "/v1/auth/refresh",
      payload: { refreshToken: login.json().refreshToken },
    });
    expect(reuseOld.statusCode).toBe(401);
    expect(reuseOld.json().error.code).toBe("INVALID_REFRESH_TOKEN");

    const logout = await app.inject({
      method: "POST",
      url: "/v1/auth/logout",
      payload: { refreshToken: refreshBody.refreshToken },
    });
    expect(logout.statusCode).toBe(200);

    const refreshAfterLogout = await app.inject({
      method: "POST",
      url: "/v1/auth/refresh",
      payload: { refreshToken: refreshBody.refreshToken },
    });
    expect(refreshAfterLogout.statusCode).toBe(401);
  });

  it("rejects a duplicate email with EMAIL_TAKEN", async () => {
    await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email: "dup@example.com", password: "longenoughpassword" } });
    const second = await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email: "dup@example.com", password: "longenoughpassword" } });
    expect(second.statusCode).toBe(409);
    expect(second.json().error.code).toBe("EMAIL_TAKEN");
  });

  it("rejects an invalid email and a short password", async () => {
    const badEmail = await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email: "not-an-email", password: "longenoughpassword" } });
    expect(badEmail.statusCode).toBe(400);
    expect(badEmail.json().error.code).toBe("VALIDATION");

    const shortPassword = await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email: "ok@example.com", password: "short" } });
    expect(shortPassword.statusCode).toBe(400);
  });

  it("rejects wrong-password login with INVALID_CREDENTIALS, not a different code", async () => {
    await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email: "someone@example.com", password: "longenoughpassword" } });
    const wrong = await app.inject({ method: "POST", url: "/v1/auth/login", payload: { email: "someone@example.com", password: "wrongpassword" } });
    expect(wrong.statusCode).toBe(401);
    expect(wrong.json().error.code).toBe("INVALID_CREDENTIALS");
  });

  it("rate-limits repeated attempts for the same ip+email", async () => {
    ({ app } = await buildTestApp());
    let lastStatus = 0;
    for (let i = 0; i < 11; i++) {
      const res = await app.inject({ method: "POST", url: "/v1/auth/login", payload: { email: "brute@example.com", password: "wrongpassword" } });
      lastStatus = res.statusCode;
    }
    expect(lastStatus).toBe(429);
  });
});
