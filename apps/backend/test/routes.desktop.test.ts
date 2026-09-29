import { generateKeyPairSync } from "node:crypto";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { FastifyInstance } from "fastify";
import { createPushSender, FcmSender, parseServiceAccount, type PushMessage, type PushSender, type PushTarget } from "../src/push.js";
import type { AppState } from "../src/state.js";
import { buildTestApp } from "./testApp.js";

async function account(app: FastifyInstance, email: string) {
  const res = await app.inject({ method: "POST", url: "/v1/auth/register", payload: { email, password: "longenoughpassword" } });
  return res.json() as { userId: string; accessToken: string };
}

async function device(app: FastifyInstance, accessToken: string, kind: "desktop" | "mobile") {
  const res = await app.inject({
    method: "POST",
    url: "/v1/devices/register",
    headers: { authorization: `Bearer ${accessToken}` },
    payload: { name: kind, kind, platform: kind === "mobile" ? "android" : "windows" },
  });
  return res.json() as { deviceId: string; deviceToken: string };
}

describe("acting on the desktop from the web", () => {
  let app: FastifyInstance;
  let state: AppState;
  beforeEach(async () => {
    ({ app, state } = await buildTestApp());
  });
  afterEach(async () => {
    await app.close();
  });

  it("says plainly when the desktop is offline", async () => {
    const { accessToken } = await account(app, "offline@example.com");
    const res = await app.inject({ method: "POST", url: "/v1/desktop/request", headers: { authorization: `Bearer ${accessToken}` }, payload: { type: "check_inbox" } });
    expect(res.statusCode).toBe(503);
    expect(res.json().error.code).toBe("DESKTOP_OFFLINE");
  });

  it("relays a request to the desktop and returns its reply", async () => {
    const { userId, accessToken } = await account(app, "relay@example.com");
    const seen: { type: string; payload: unknown }[] = [];
    // A stand-in desktop connection that answers like the real one.
    state.hub.register(userId, "desktop-1", "desktop", {
      send: (text) => {
        const frame = JSON.parse(text) as { id: string; type: string; payload: unknown };
        seen.push({ type: frame.type, payload: frame.payload });
        const reply =
          frame.type === "answer_application_questions"
            ? { ok: true, data: { id: "app-1", status: "APPLYING" } }
            : { ok: false, error: { code: "AGENT_BUSY", message: "agent is APPLYING" } };
        setTimeout(() => state.hub.takeDesktopReply(JSON.stringify({ v: 1, id: frame.id, type: "response", payload: reply })), 5);
      },
      close: () => undefined,
    });
    const auth = { authorization: `Bearer ${accessToken}` };
    const answer = await app.inject({
      method: "POST",
      url: "/v1/desktop/request",
      headers: auth,
      payload: { type: "answer_application_questions", payload: { id: "app-1", answers: [{ question: "Notice period?", answer: "30 days" }] } },
    });
    expect(answer.statusCode).toBe(200);
    expect(answer.json().data).toEqual({ id: "app-1", status: "APPLYING" });
    expect(seen[0]).toEqual({ type: "answer_application_questions", payload: { id: "app-1", answers: [{ question: "Notice period?", answer: "30 days" }] } });

    const busy = await app.inject({ method: "POST", url: "/v1/desktop/request", headers: auth, payload: { type: "start_job_hunt" } });
    expect(busy.statusCode).toBe(409);
    expect(busy.json().error).toEqual({ code: "AGENT_BUSY", message: "agent is APPLYING" });

    const bad = await app.inject({ method: "POST", url: "/v1/desktop/request", headers: auth, payload: { type: "Drop Table" } });
    expect(bad.statusCode).toBe(400);
  });

  it("answers from the server are not forwarded to phones", async () => {
    const { userId } = await account(app, "noleak@example.com");
    const toPhones: string[] = [];
    state.hub.register(userId, "phone-1", "mobile", { send: (t) => void toPhones.push(t), close: () => undefined });
    state.hub.register(userId, "desktop-1", "desktop", {
      send: (text) => {
        const { id } = JSON.parse(text) as { id: string };
        state.hub.takeDesktopReply(JSON.stringify({ v: 1, id, type: "response", payload: { ok: true, data: 1 } }));
      },
      close: () => undefined,
    });
    await state.hub.request(userId, "get_agent_status", {});
    expect(toPhones.filter((t) => JSON.parse(t).type === "response")).toHaveLength(0);
  });
});

describe("push notifications to phones", () => {
  let app: FastifyInstance;
  let state: AppState;
  const sent: { targets: PushTarget[]; message: PushMessage }[] = [];
  beforeEach(async () => {
    ({ app, state } = await buildTestApp());
    sent.length = 0;
    const fake: PushSender = {
      async send(targets, message) {
        sent.push({ targets, message });
        return targets.filter((t) => t.token.startsWith("dead")).map((t) => t.deviceId);
      },
    };
    state.push = fake;
  });
  afterEach(async () => {
    await app.close();
  });

  it("registers a phone's token with its own device token only", async () => {
    const { accessToken } = await account(app, "token@example.com");
    const phone = await device(app, accessToken, "mobile");
    const put = (auth: string, token: unknown) =>
      app.inject({ method: "PUT", url: "/v1/devices/me/push-token", headers: { authorization: `Bearer ${auth}` }, payload: { token } });
    expect((await put(accessToken, "fcm-token-123456")).statusCode).toBe(401);
    expect((await put(phone.deviceToken, "short")).statusCode).toBe(400);
    expect((await put(phone.deviceToken, "fcm-token-123456")).statusCode).toBe(200);
    const { userId } = (await app.inject({ method: "GET", url: "/v1/devices", headers: { authorization: `Bearer ${phone.deviceToken}` } })).json().devices[0];
    expect(await state.db.listPushTokens(userId)).toEqual([{ deviceId: phone.deviceId, token: "fcm-token-123456" }]);
    const listing = await app.inject({ method: "GET", url: "/v1/devices", headers: { authorization: `Bearer ${phone.deviceToken}` } });
    expect(JSON.stringify(listing.json())).not.toContain("fcm-token-123456");
  });

  it("pushes each new notification the desktop syncs, and forgets dead tokens", async () => {
    const { accessToken } = await account(app, "pushes@example.com");
    const desktop = await device(app, accessToken, "desktop");
    const phone = await device(app, accessToken, "mobile");
    const oldPhone = await device(app, accessToken, "mobile");
    const setToken = (d: { deviceToken: string }, token: string) =>
      app.inject({ method: "PUT", url: "/v1/devices/me/push-token", headers: { authorization: `Bearer ${d.deviceToken}` }, payload: { token } });
    await setToken(phone, "live-token-123456");
    await setToken(oldPhone, "dead-token-123456");

    const doc = { id: "n-1", level: "SUCCESS", kind: "APPLICATION_REPLY", title: "Brightpath wants to interview you", body: "Interview invitation", linkPage: "application", linkId: "app-1", read: false, createdAt: "2026-09-29T10:00:00Z" };
    await app.inject({ method: "PUT", url: "/v1/data/notifications/n-1", headers: { authorization: `Bearer ${desktop.deviceToken}` }, payload: doc });
    await new Promise((r) => setTimeout(r, 20));
    expect(sent).toHaveLength(1);
    expect(sent[0]!.targets.map((t) => t.token).sort()).toEqual(["dead-token-123456", "live-token-123456"]);
    expect(sent[0]!.message).toMatchObject({ title: "Brightpath wants to interview you", data: { linkPage: "application", linkId: "app-1" } });

    const devices = (await app.inject({ method: "GET", url: "/v1/devices", headers: { authorization: `Bearer ${desktop.deviceToken}` } })).json().devices as { userId: string }[];
    const tokens = await state.db.listPushTokens(devices[0]!.userId);
    expect(tokens.map((t) => t.token)).toEqual(["live-token-123456"]);

    // Marked read on the desktop: synced again, not pushed again.
    await app.inject({ method: "PUT", url: "/v1/data/notifications/n-1", headers: { authorization: `Bearer ${desktop.deviceToken}` }, payload: { ...doc, read: true } });
    await new Promise((r) => setTimeout(r, 20));
    expect(sent).toHaveLength(1);
  });
});

describe("Firebase sender", () => {
  const { privateKey } = generateKeyPairSync("rsa", { modulusLength: 2048, privateKeyEncoding: { type: "pkcs8", format: "pem" }, publicKeyEncoding: { type: "spki", format: "pem" } });
  const sa = { project_id: "job-hunter-test", client_email: "push@job-hunter-test.iam.gserviceaccount.com", private_key: privateKey };

  it("reads a key file relative to the repository root, whatever the working directory", () => {
    const root = mkdtempSync(path.join(tmpdir(), "jh-root-"));
    writeFileSync(path.join(root, "service-account.json"), JSON.stringify(sa));
    const cwd = process.cwd();
    process.chdir(tmpdir()); // like PM2 running the server from apps/backend
    try {
      // Exactly the user's .env value, quotes included.
      expect(parseServiceAccount('"./service-account.json"', root)?.project_id).toBe("job-hunter-test");
      expect(parseServiceAccount("service-account.json", root)?.project_id).toBe("job-hunter-test");
      expect(() => parseServiceAccount("./missing.json", root)).toThrow(/doesn't exist/);
    } finally {
      process.chdir(cwd);
    }
  });

  it("a bad push setting turns push off instead of stopping the server", () => {
    expect(createPushSender("./missing.json")).toBeNull();
    expect(createPushSender("{not json")).toBeNull();
    expect(createPushSender(JSON.stringify(sa))).not.toBeNull();
  });

  it("reads the service account as JSON or base64", () => {
    expect(parseServiceAccount(JSON.stringify(sa))?.project_id).toBe("job-hunter-test");
    expect(parseServiceAccount(Buffer.from(JSON.stringify(sa)).toString("base64"))?.client_email).toBe(sa.client_email);
    expect(parseServiceAccount("")).toBeNull();
    expect(() => parseServiceAccount("{}")).toThrow();
  });

  it("signs in once, sends to each token and reports unregistered ones", async () => {
    const calls: { url: string; body: string }[] = [];
    const fakeFetch = (async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), body: String(init?.body ?? "") });
      if (String(url).startsWith("https://oauth2.googleapis.com/token")) return new Response(JSON.stringify({ access_token: "ya29.test", expires_in: 3600 }), { status: 200 });
      const body = String(init?.body ?? "");
      if (body.includes("gone-token")) return new Response(JSON.stringify({ error: { status: "NOT_FOUND", details: [{ errorCode: "UNREGISTERED" }] } }), { status: 404 });
      return new Response("{}", { status: 200 });
    }) as typeof fetch;
    const sender = new FcmSender(sa, fakeFetch);
    const message = { title: "Hello", body: "World", data: { id: "n-1" } };
    const invalid = await sender.send([{ deviceId: "d1", token: "good-token" }, { deviceId: "d2", token: "gone-token" }], message);
    expect(invalid).toEqual(["d2"]);
    await sender.send([{ deviceId: "d1", token: "good-token" }], message);
    const auth = calls.filter((c) => c.url.startsWith("https://oauth2.googleapis.com/token"));
    expect(auth).toHaveLength(1);
    expect(auth[0]!.body).toContain("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer");
    const push = calls.find((c) => c.url.includes("/projects/job-hunter-test/messages:send"))!;
    expect(JSON.parse(push.body).message).toMatchObject({ token: "good-token", notification: { title: "Hello", body: "World" }, android: { priority: "high" } });
  });
});
