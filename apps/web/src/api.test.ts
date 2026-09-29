import { describe, expect, it, vi } from "vitest";

import { ApiClient, ApiError, desktopErrorMessage, type Session } from "./api";
import { collectAnswers, questionInput } from "./questions";
import { parseRoute, href } from "./route";
import { countByGroup, trackingStatuses } from "./statusGroups";
import type { ApplicationListItem, ApplicationStatus, Notification, PendingQuestion } from "@job-hunter/types";
import { notificationHref, takeFresh, unreadCount } from "./notifications";

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

  it("creates an account through /v1/auth/register and signs straight in", async () => {
    const calls: Call[] = [];
    const sessions: (Session | null)[] = [];
    const fetchImpl = async (url: string, init?: RequestInit) => {
      calls.push({ url, init });
      return json(200, { userId: "u1", accessToken: "a", refreshToken: "r" });
    };
    const api = new ApiClient(null, (s) => sessions.push(s), fetchImpl);
    await api.register(" New@Example.com ", "longenoughpassword");
    expect(calls[0]!.url).toBe("/v1/auth/register");
    expect(JSON.parse(String(calls[0]!.init?.body))).toEqual({ email: " New@Example.com ", password: "longenoughpassword" });
    expect(sessions.at(-1)).toEqual({ email: "new@example.com", accessToken: "a", refreshToken: "r" });
  });

  it("surfaces the backend's error message on a failed login", async () => {
    const fetchImpl = async () => json(401, { error: { code: "INVALID_CREDENTIALS", message: "Wrong email or password." } });
    const api = new ApiClient(null, () => undefined, fetchImpl);
    await expect(api.login("a@example.com", "bad")).rejects.toMatchObject({ code: "INVALID_CREDENTIALS", message: "Wrong email or password." });
  });
});

describe("ApiClient.desktop", () => {
  it("posts the request type and payload and returns the desktop's data", async () => {
    const calls: Call[] = [];
    const fetchImpl = async (url: string, init?: RequestInit) => {
      calls.push({ url, init });
      return json(200, { data: { id: "app1", status: "APPLYING" } });
    };
    const api = new ApiClient(SESSION, () => undefined, fetchImpl);
    const answers = [{ question: "Notice period?", answer: "30 days" }];
    await expect(api.answerApplicationQuestions("app1", answers)).resolves.toEqual({ id: "app1", status: "APPLYING" });
    expect(calls[0]!.url).toBe("/v1/desktop/request");
    expect(calls[0]!.init?.method).toBe("POST");
    const headers = calls[0]!.init?.headers as Record<string, string>;
    expect(headers.Authorization).toBe("Bearer old-access");
    expect(headers["Content-Type"]).toBe("application/json");
    expect(JSON.parse(String(calls[0]!.init?.body))).toEqual({ type: "answer_application_questions", payload: { id: "app1", answers } });
  });

  it("sends an empty payload for requests that take none", async () => {
    const bodies: unknown[] = [];
    const fetchImpl = async (_url: string, init?: RequestInit) => {
      bodies.push(JSON.parse(String(init?.body)));
      return json(200, { data: { runId: "r1" } });
    };
    const api = new ApiClient(SESSION, () => undefined, fetchImpl);
    await expect(api.checkInbox()).resolves.toEqual({ runId: "r1" });
    await api.startJobHunt({ sources: ["LinkedIn Posts"] });
    expect(bodies).toEqual([
      { type: "check_inbox", payload: {} },
      { type: "start_job_hunt", payload: { sources: ["LinkedIn Posts"] } },
    ]);
  });

  it("refreshes an expired token before retrying the same request", async () => {
    const calls: string[] = [];
    const fetchImpl = async (url: string, init?: RequestInit) => {
      calls.push(url);
      if (url === "/v1/auth/refresh") return json(200, { accessToken: "new-access", refreshToken: "new-refresh" });
      const auth = (init?.headers as Record<string, string>).Authorization;
      return auth === "Bearer new-access" ? json(200, { data: { stopped: true } }) : json(401, { error: { code: "UNAUTHORIZED", message: "expired" } });
    };
    const api = new ApiClient(SESSION, () => undefined, fetchImpl);
    await expect(api.stopJobHunt()).resolves.toEqual({ stopped: true });
    expect(calls).toEqual(["/v1/desktop/request", "/v1/auth/refresh", "/v1/desktop/request"]);
  });

  it("explains an offline desktop in plain words", async () => {
    const fetchImpl = async () => json(503, { error: { code: "DESKTOP_OFFLINE", message: "Your desktop is not online right now." } });
    const api = new ApiClient(SESSION, () => undefined, fetchImpl);
    const err = await api.approveApplication("app1").catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err).toMatchObject({
      code: "DESKTOP_OFFLINE",
      status: 503,
      message: "Your desktop is offline. Open Job Hunter on your computer and try again.",
    });
  });

  it("explains a busy agent and a stale application (409)", async () => {
    let reply = { code: "AGENT_BUSY", message: "agent busy: JOB_HUNT" };
    const fetchImpl = async () => json(409, { error: reply });
    const api = new ApiClient(SESSION, () => undefined, fetchImpl);
    await expect(api.startJobHunt()).rejects.toMatchObject({
      status: 409,
      code: "AGENT_BUSY",
      message: "The agent on your desktop is busy with another task. Try again once it finishes.",
    });
    reply = { code: "INVALID_TRANSITION", message: "cannot move APPLIED to APPROVED" };
    await expect(api.approveApplication("app1")).rejects.toMatchObject({
      status: 409,
      code: "INVALID_TRANSITION",
      message: expect.stringContaining("Refresh to see where it stands"),
    });
  });

  it("keeps the desktop's own message for other errors, and explains network failures", async () => {
    const api = new ApiClient(SESSION, () => undefined, async () =>
      json(400, { error: { code: "VALIDATION", message: "Nothing was sent yet, so there are no replies to look for." } }),
    );
    await expect(api.checkInbox()).rejects.toMatchObject({ code: "VALIDATION", message: "Nothing was sent yet, so there are no replies to look for." });

    const offline = new ApiClient(SESSION, () => undefined, async () => {
      throw new TypeError("Failed to fetch");
    });
    await expect(offline.checkInbox()).rejects.toMatchObject({ code: "NETWORK", message: expect.stringContaining("Couldn't reach the server") });
    expect(desktopErrorMessage("OTHER", "  ")).toBe("Your desktop couldn't do that.");
  });
});

describe("pending questions", () => {
  const q = (id: string, required: boolean, fieldType = "text", options: string[] = []): PendingQuestion => ({
    id,
    question: `Question ${id}?`,
    fieldType,
    options,
    required,
    context: "",
  });

  it("sends trimmed answers and reports required ones left blank", () => {
    const questions = [q("a", true), q("b", false), q("c", true), q("d", false)];
    const { answers, missing } = collectAnswers(questions, { a: "  12 LPA ", b: "", c: "   ", d: "Yes" });
    expect(answers).toEqual([
      { question: "Question a?", answer: "12 LPA" },
      { question: "Question d?", answer: "Yes" },
    ]);
    expect(missing).toEqual(["c"]);
  });

  it("picks an input for each field type, falling back to text", () => {
    expect(questionInput(q("a", true, "select", ["Yes", "No"]))).toBe("select");
    expect(questionInput(q("a", true, "radio", ["Yes", "No"]))).toBe("radio");
    expect(questionInput(q("a", true, "radio"))).toBe("text");
    expect(questionInput(q("a", true, "TextArea"))).toBe("textarea");
    expect(questionInput(q("a", true, "number"))).toBe("number");
    expect(questionInput(q("a", true, "date"))).toBe("date");
    expect(questionInput(q("a", true, "checkbox"))).toBe("text");
  });
});

describe("tracking statuses", () => {
  it("offers only the moves the desktop allows", () => {
    expect(trackingStatuses("APPLIED")).toEqual(["INTERVIEW", "OFFER", "REJECTED", "WITHDRAWN"]);
    expect(trackingStatuses("INTERVIEW")).toEqual(["OFFER", "REJECTED", "WITHDRAWN"]);
    expect(trackingStatuses("READY_FOR_REVIEW")).toEqual([]);
    expect(trackingStatuses("WITHDRAWN")).toEqual([]);
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

describe("notifications", () => {
  const n = (id: string, createdAt: string, linkPage = "applications", linkId: string | null = null) =>
    ({ id, createdAt, linkPage, linkId, title: id, body: "", level: "INFO", kind: "X", read: false, userId: "u", updatedAt: createdAt }) as Notification;

  it("counts what arrived after the list was last opened", () => {
    const list = [n("a", "2026-09-28T10:00:00Z"), n("b", "2026-09-28T11:00:00Z")];
    expect(unreadCount(list, "")).toBe(2);
    expect(unreadCount(list, "2026-09-28T10:00:00Z")).toBe(1);
  });

  it("alerts only for new arrivals, never replaying history on first load", () => {
    const memory = new Map<string, string>();
    vi.stubGlobal("localStorage", { getItem: (k: string) => memory.get(k) ?? null, setItem: (k: string, v: string) => void memory.set(k, v) });
    expect(takeFresh([n("a", "2026-09-28T10:00:00Z")])).toEqual([]);
    const fresh = takeFresh([n("a", "2026-09-28T10:00:00Z"), n("c", "2026-09-28T12:00:00Z"), n("b", "2026-09-28T11:00:00Z")]);
    expect(fresh.map((x) => x.id)).toEqual(["b", "c"]);
    expect(takeFresh([n("c", "2026-09-28T12:00:00Z")])).toEqual([]);
  });

  it("links to web pages when there is one", () => {
    expect(notificationHref(n("a", "t", "application", "app 1"))).toBe("#/applications/app%201");
    expect(notificationHref(n("a", "t", "applications"))).toBe("#/applications");
    expect(notificationHref(n("a", "t", "job-sites"))).toBeNull();
  });
});
