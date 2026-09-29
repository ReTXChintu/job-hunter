/**
 * The generic domain-data API. Every operation here mirrors
 * `crates/job-hunter-core/src/store/mongo.rs`'s `upsert`/`delete`/`fetch_all`
 * exactly (same three operations, same semantics: `_id` = the document's
 * own `id`, scoped by account) so the Rust sync layer can eventually swap
 * its direct MongoDB driver calls for HTTP calls to this backend with no
 * change in behaviour -- see docs/relay.md and docs/mobile.md for the
 * phased rollout this is part of.
 *
 * A `changed` frame is broadcast over the WebSocket hub after every write,
 * so any connected mobile app knows to refetch instead of polling.
 */
import type { FastifyInstance } from "fastify";

import { Errors } from "../errors.js";
import { isDomainCollection } from "../models.js";
import { requireAuth } from "../plugins/auth.js";
import type { AppState } from "../state.js";

function assertCollection(name: string): void {
  if (!isDomainCollection(name)) {
    throw Errors.validation(`unknown collection "${name}"`);
  }
}

function broadcastChanged(state: AppState, userId: string, collection: string): void {
  const frame = JSON.stringify({
    v: 1,
    id: crypto.randomUUID(),
    type: "changed",
    payload: { collections: [collection] },
  });
  state.hub.routeToMobiles(userId, frame);
}

/** Push a new, unread notification to the account's phones right away, so
 * they can show it even though the desktop only syncs it here. */
function pushNotification(state: AppState, userId: string, doc: Record<string, unknown>): void {
  if (doc.read === true) return;
  const pick = (key: string) => (typeof doc[key] === "string" ? doc[key] : "");
  const frame = JSON.stringify({
    v: 1,
    id: crypto.randomUUID(),
    type: "notification",
    payload: {
      id: pick("id"),
      level: pick("level"),
      kind: pick("kind"),
      title: pick("title"),
      body: pick("body"),
      linkPage: pick("linkPage"),
      linkId: typeof doc.linkId === "string" ? doc.linkId : null,
      createdAt: pick("createdAt"),
    },
  });
  state.hub.routeToMobiles(userId, frame);
  void sendPush(state, userId, doc);
}

/** The same notification as a Firebase push, which reaches phones whose
 * app is closed. Best-effort: a failure never fails the sync request. */
async function sendPush(state: AppState, userId: string, doc: Record<string, unknown>): Promise<void> {
  if (!state.push) return;
  const text = (key: string) => (typeof doc[key] === "string" ? (doc[key] as string) : "");
  try {
    const targets = await state.db.listPushTokens(userId);
    const invalid = await state.push.send(targets, {
      title: text("title") || "Job Hunter",
      body: text("body"),
      data: { id: text("id"), kind: text("kind"), level: text("level"), linkPage: text("linkPage"), linkId: text("linkId"), createdAt: text("createdAt") },
    });
    await Promise.all(invalid.map((deviceId) => state.db.setDevicePushToken(userId, deviceId, null)));
  } catch (e) {
    console.warn(`push notification failed: ${(e as Error).message}`);
  }
}

export function registerDataRoutes(app: FastifyInstance, state: AppState): void {
  app.put<{ Params: { collection: string; id: string }; Body: Record<string, unknown> }>(
    "/v1/data/:collection/:id",
    async (request, reply) => {
      const userId = await requireAuth(state, request, reply);
      if (!userId) return;
      assertCollection(request.params.collection);
      await state.db.upsertDoc(request.params.collection, userId, request.params.id, request.body ?? {});
      broadcastChanged(state, userId, request.params.collection);
      if (request.params.collection === "notifications") pushNotification(state, userId, request.body ?? {});
      reply.send({});
    },
  );

  app.delete<{ Params: { collection: string; id: string } }>("/v1/data/:collection/:id", async (request, reply) => {
    const userId = await requireAuth(state, request, reply);
    if (!userId) return;
    assertCollection(request.params.collection);
    await state.db.deleteDoc(request.params.collection, userId, request.params.id);
    broadcastChanged(state, userId, request.params.collection);
    reply.send({});
  });

  app.get<{ Params: { collection: string } }>("/v1/data/:collection", async (request, reply) => {
    const userId = await requireAuth(state, request, reply);
    if (!userId) return;
    assertCollection(request.params.collection);
    const docs = await state.db.fetchAllDocs(request.params.collection, userId);
    reply.send({ documents: docs });
  });

  // ---- Convenience read views -------------------------------------------------
  // Joined, ready-to-render shapes matching @job-hunter/types'
  // `ApplicationListItem`/`ApplicationDetail`, so the mobile app can read
  // application/job/analysis data directly from the backend -- without a
  // live desktop connection -- exactly the capability this backend exists
  // to add. Resume/cover-letter *file content* stays desktop-only (see
  // docs/mobile.md); this only returns the small metadata fields already
  // on the domain records.

  app.get("/v1/applications", async (request, reply) => {
    const userId = await requireAuth(state, request, reply);
    if (!userId) return;
    const applications = await state.db.fetchAllDocs("applications", userId);
    const items = await Promise.all(applications.map((application) => joinApplication(state, userId, application)));
    reply.send(items.filter((i) => i !== null));
  });

  app.get<{ Params: { id: string } }>("/v1/applications/:id", async (request, reply) => {
    const userId = await requireAuth(state, request, reply);
    if (!userId) return;
    const application = await state.db.fetchDoc("applications", userId, request.params.id);
    if (!application) throw Errors.notFound("application not found");
    const item = await joinApplication(state, userId, application);
    if (!item) throw Errors.notFound("application's job could not be found");
    reply.send(item);
  });
}

async function joinApplication(state: AppState, userId: string, application: Record<string, unknown>) {
  const jobId = application.jobId as string | undefined;
  if (!jobId) return null;
  const job = await state.db.fetchDoc("jobs", userId, jobId);
  if (!job) return null;
  const analyses = await state.db.fetchDocsByField("job_analyses", userId, "jobId", jobId);
  const analysis = analyses[0] ?? null;
  return { application, job, analysis };
}
