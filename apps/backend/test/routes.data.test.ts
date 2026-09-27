import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { FastifyInstance } from "fastify";
import type { AppState } from "../src/state.js";
import { buildTestApp } from "./testApp.js";

async function registerAndGetAccessToken(app: FastifyInstance, email: string) {
  const res = await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email, password: "longenoughpassword" } });
  return res.json().accessToken as string;
}

describe("generic data routes", () => {
  let app: FastifyInstance;

  beforeEach(async () => {
    ({ app } = await buildTestApp());
  });
  afterEach(async () => {
    await app.close();
  });

  it("upserts, lists and deletes a document, scoped to the account", async () => {
    const token = await registerAndGetAccessToken(app, "data@example.com");
    const auth = { authorization: `Bearer ${token}` };

    const put = await app.inject({ method: "PUT", url: "/v1/data/jobs/job-1", headers: auth, payload: { id: "job-1", title: "Staff Engineer", company: "Acme" } });
    expect(put.statusCode).toBe(200);

    const list = await app.inject({ method: "GET", url: "/v1/data/jobs", headers: auth });
    expect(list.statusCode).toBe(200);
    expect(list.json().documents).toHaveLength(1);
    expect(list.json().documents[0].title).toBe("Staff Engineer");

    const del = await app.inject({ method: "DELETE", url: "/v1/data/jobs/job-1", headers: auth });
    expect(del.statusCode).toBe(200);

    const listAfter = await app.inject({ method: "GET", url: "/v1/data/jobs", headers: auth });
    expect(listAfter.json().documents).toHaveLength(0);
  });

  it("rejects an unknown collection name", async () => {
    const token = await registerAndGetAccessToken(app, "data2@example.com");
    const res = await app.inject({ method: "GET", url: "/v1/data/not_a_real_collection", headers: { authorization: `Bearer ${token}` } });
    expect(res.statusCode).toBe(400);
    expect(res.json().error.code).toBe("VALIDATION");
  });

  it("scopes documents to the account -- one account never sees another's data", async () => {
    const tokenA = await registerAndGetAccessToken(app, "a@example.com");
    const tokenB = await registerAndGetAccessToken(app, "b@example.com");

    await app.inject({ method: "PUT", url: "/v1/data/jobs/job-1", headers: { authorization: `Bearer ${tokenA}` }, payload: { id: "job-1", title: "A's job" } });

    const listForB = await app.inject({ method: "GET", url: "/v1/data/jobs", headers: { authorization: `Bearer ${tokenB}` } });
    expect(listForB.json().documents).toHaveLength(0);

    const listForA = await app.inject({ method: "GET", url: "/v1/data/jobs", headers: { authorization: `Bearer ${tokenA}` } });
    expect(listForA.json().documents).toHaveLength(1);
  });

  it("stamps the authenticated userId onto a document even if the body claims a different one", async () => {
    const tokenA = await registerAndGetAccessToken(app, "stamp-a@example.com");
    const tokenB = await registerAndGetAccessToken(app, "stamp-b@example.com");
    // Decode B's user id from a request as A tries to write data claiming to be B.
    const meB = await app.inject({ method: "GET", url: "/v1/devices", headers: { authorization: `Bearer ${tokenB}` } });
    expect(meB.statusCode).toBe(200);

    await app.inject({
      method: "PUT",
      url: "/v1/data/jobs/job-x",
      headers: { authorization: `Bearer ${tokenA}` },
      payload: { id: "job-x", userId: "someone-elses-id", title: "Sneaky" },
    });

    const listForA = await app.inject({ method: "GET", url: "/v1/data/jobs", headers: { authorization: `Bearer ${tokenA}` } });
    expect(listForA.json().documents).toHaveLength(1);
    const listForB = await app.inject({ method: "GET", url: "/v1/data/jobs", headers: { authorization: `Bearer ${tokenB}` } });
    expect(listForB.json().documents).toHaveLength(0);
  });
});

describe("application read views", () => {
  let app: FastifyInstance;
  let state: AppState;

  beforeEach(async () => {
    ({ app, state } = await buildTestApp());
  });
  afterEach(async () => {
    await app.close();
  });

  it("joins an application with its job and analysis for /v1/applications", async () => {
    const token = await registerAndGetAccessToken(app, "join@example.com");
    const auth = { authorization: `Bearer ${token}` };

    await app.inject({ method: "PUT", url: "/v1/data/jobs/job-1", headers: auth, payload: { id: "job-1", title: "Staff Engineer", company: "Acme" } });
    await app.inject({ method: "PUT", url: "/v1/data/job_analyses/analysis-1", headers: auth, payload: { id: "analysis-1", jobId: "job-1", matchScore: 92 } });
    await app.inject({ method: "PUT", url: "/v1/data/applications/app-1", headers: auth, payload: { id: "app-1", jobId: "job-1", status: "READY_FOR_REVIEW" } });

    const list = await app.inject({ method: "GET", url: "/v1/applications", headers: auth });
    expect(list.statusCode).toBe(200);
    const items = list.json();
    expect(items).toHaveLength(1);
    expect(items[0].application.status).toBe("READY_FOR_REVIEW");
    expect(items[0].job.title).toBe("Staff Engineer");
    expect(items[0].analysis.matchScore).toBe(92);

    const detail = await app.inject({ method: "GET", url: "/v1/applications/app-1", headers: auth });
    expect(detail.statusCode).toBe(200);
    expect(detail.json().job.company).toBe("Acme");

    const missing = await app.inject({ method: "GET", url: "/v1/applications/no-such-id", headers: auth });
    expect(missing.statusCode).toBe(404);
  });

  it("omits an application whose job no longer exists rather than erroring the whole list", async () => {
    const token = await registerAndGetAccessToken(app, "orphan@example.com");
    const auth = { authorization: `Bearer ${token}` };
    await app.inject({ method: "PUT", url: "/v1/data/applications/orphan-app", headers: auth, payload: { id: "orphan-app", jobId: "does-not-exist", status: "READY_FOR_REVIEW" } });

    const list = await app.inject({ method: "GET", url: "/v1/applications", headers: auth });
    expect(list.statusCode).toBe(200);
    expect(list.json()).toHaveLength(0);
  });

  it("pushes a new notification to the account's phones as soon as the desktop syncs it", async () => {
    const token = await registerAndGetAccessToken(app, "notify@example.com");
    const auth = { authorization: `Bearer ${token}` };
    const sent: string[] = [];
    vi.spyOn(state.hub, "routeToMobiles").mockImplementation((_user: string, text: string) => void sent.push(text));

    const doc = { id: "n-1", level: "ERROR", kind: "PROFILE_UPDATE_FAILED", title: "LinkedIn profile update stopped", body: "Claude made no progress", linkPage: "job-sites", linkId: "linkedin", read: false, createdAt: "2026-09-28T00:00:00Z" };
    expect((await app.inject({ method: "PUT", url: "/v1/data/notifications/n-1", headers: auth, payload: doc })).statusCode).toBe(200);
    const pushed = sent.map((t) => JSON.parse(t)).filter((f) => f.type === "notification");
    expect(pushed).toHaveLength(1);
    expect(pushed[0].payload).toMatchObject({ id: "n-1", title: "LinkedIn profile update stopped", linkPage: "job-sites" });

    // Marking it read on the desktop syncs again but doesn't re-notify.
    await app.inject({ method: "PUT", url: "/v1/data/notifications/n-1", headers: auth, payload: { ...doc, read: true } });
    expect(sent.map((t) => JSON.parse(t)).filter((f) => f.type === "notification")).toHaveLength(1);

    const list = await app.inject({ method: "GET", url: "/v1/data/notifications", headers: auth });
    expect(list.json().documents[0].title).toBe("LinkedIn profile update stopped");
  });
});
