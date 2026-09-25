import { describe, expect, it } from "vitest";

import { ApiClient, ApiError, type Session } from "./api";
import { parseRoute, href } from "./route";
import { countByGroup } from "./statusGroups";
import type { ApplicationListItem, ApplicationStatus } from "@job-hunter/types";

type Call = { url: string; init?: RequestInit };

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } });
}

const SESSION: Session = { email: "a@example.com", accessToken: "old-access", refreshToken: "old-refresh" };

describe("ApiClient", () => {
  it("refreshes an expired access token once and retries the request", async () => {
    const calls: Call[] = [];
    const sessions: (Session | null)[] = [];
    const fetchImpl = async (url: string, init?: RequestInit) => {
      calls.push({ url, init });
      if (url === "/v1/auth/refresh") return json(200, { accessToken: "new-access", refreshToken: "new-refresh" });
      const auth = (init?.headers as Record<string, string> | undefined)?.Authorization;
      return auth === "Bearer new-access" ? json(200, []) : json(401, { error: { code: "UNAUTHORIZED", message: "expired" } });
    };
    const api = new ApiClient(SESSION, (s) => sessions.push(s), fetchImpl);

    await expect(api.applications()).resolves.toEqual([]);
    expect(calls.map((c) => c.url)).toEqual(["/v1/applications", "/v1/auth/refresh", "/v1/applications"]);
    expect(JSON.parse(String(calls[1]!.init?.body))).toEqual({ refreshToken: "old-refresh" });
    expect(sessions.at(-1)).toEqual({ email: "a@example.com", accessToken: "new-access", refreshToken: "new-refresh" });
  });

  it("shares one refresh between concurrent requests", async () => {
    let refreshes = 0;
    const fetchImpl = async (url: string, init?: RequestInit) => {
      if (url === "/v1/auth/refresh") {
        refreshes += 1;
        return json(200, { accessToken: "new-access", refreshToken: "new-refresh" });
      }
      const auth = (init?.headers as Record<string, string> | undefined)?.Authorization;
      return auth === "Bearer new-access" ? json(200, { documents: [] }) : json(401, { error: { code: "UNAUTHORIZED", message: "expired" } });
    };
    const api = new ApiClient(SESSION, () => undefined, fetchImpl);
    await Promise.all([api.jobs(), api.analyses(), api.jobs()]);
    expect(refreshes).toBe(1);
  });

  it("ends the session when the refresh token is rejected too", async () => {
    const sessions: (Session | null)[] = [];
    const fetchImpl = async () => json(401, { error: { code: "UNAUTHORIZED", message: "nope" } });
    const api = new ApiClient(SESSION, (s) => sessions.push(s), fetchImpl);
    await expect(api.applications()).rejects.toBeInstanceOf(ApiError);
    expect(sessions.at(-1)).toBeNull();
    expect(api.current).toBeNull();
  });

  it("surfaces the backend's error message on a failed login", async () => {
    const fetchImpl = async () => json(401, { error: { code: "INVALID_CREDENTIALS", message: "Wrong email or password." } });
    const api = new ApiClient(null, () => undefined, fetchImpl);
    await expect(api.login("a@example.com", "bad")).rejects.toMatchObject({ code: "INVALID_CREDENTIALS", message: "Wrong email or password." });
  });
});

describe("routes", () => {
  it("round-trips every route through the hash", () => {
    for (const route of [
      { page: "overview" },
      { page: "applications" },
      { page: "applications", group: "review" },
      { page: "application", id: "abc 123" },
      { page: "jobs" },
    ] as const) {
      expect(parseRoute(href(route))).toEqual(route);
    }
    expect(parseRoute("")).toEqual({ page: "overview" });
    expect(parseRoute("#/nonsense")).toEqual({ page: "overview" });
  });
});

describe("status groups", () => {
  it("counts each application in its group and in All", () => {
    const item = (status: ApplicationStatus) => ({ application: { status } }) as unknown as ApplicationListItem;
    const counts = countByGroup([item("READY_FOR_REVIEW"), item("WAITING_FOR_USER"), item("MANUAL_ACTION_REQUIRED"), item("OFFER")]);
    expect(counts).toMatchObject({ all: 4, review: 1, attention: 2, interviews: 1, applied: 0 });
  });
});
