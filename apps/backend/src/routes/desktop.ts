/**
 * Acting on the desktop from anywhere, and push-token registration.
 *
 * `POST /v1/desktop/request` lets an HTTP client (the web app) send the same
 * requests a phone sends over the WebSocket (see docs/mobile-protocol.md).
 * The server only relays: the desktop runs every request through the same
 * approval-gated code as its own UI and replies, and this returns that reply.
 */
import type { FastifyInstance } from "fastify";

import { sha256Hex } from "../auth.js";
import { Errors } from "../errors.js";
import { requireAuth } from "../plugins/auth.js";
import type { AppState } from "../state.js";

const REPLY_STATUS: Record<string, number> = {
  DESKTOP_OFFLINE: 503,
  DESKTOP_TIMEOUT: 504,
  NOT_FOUND: 404,
  AGENT_BUSY: 409,
  INVALID_TRANSITION: 409,
};

export function registerDesktopRoutes(app: FastifyInstance, state: AppState): void {
  app.post<{ Body: { type?: unknown; payload?: unknown } }>("/v1/desktop/request", async (request, reply) => {
    const userId = await requireAuth(state, request, reply);
    if (!userId) return;
    const type = request.body?.type;
    if (typeof type !== "string" || !/^[a-z_]{1,64}$/.test(type)) {
      throw Errors.validation("type must be a request type like \"answer_application_questions\"");
    }
    const result = await state.hub.request(userId, type, request.body?.payload ?? {});
    if (result.ok) {
      reply.send({ data: result.data ?? null });
      return;
    }
    const error = result.error ?? { code: "OTHER", message: "The desktop couldn't do that." };
    reply.status(REPLY_STATUS[error.code] ?? 400).send({ error });
  });

  /** A phone registers (or, with null, clears) its Firebase push token. Only
   * with the phone's own device token, so it's stored on the right device. */
  app.put<{ Body: { token?: unknown } }>("/v1/devices/me/push-token", async (request, reply) => {
    const header = request.headers.authorization;
    const bearer = header?.startsWith("Bearer ") ? header.slice("Bearer ".length) : "";
    const device = bearer ? await state.db.findDeviceByTokenHash(sha256Hex(bearer)) : null;
    if (!device) {
      const err = Errors.unauthorized();
      reply.status(err.status).send(err.toBody());
      return;
    }
    const token = request.body?.token;
    if (token !== null && (typeof token !== "string" || token.length < 10 || token.length > 4096)) {
      throw Errors.validation("token must be a Firebase registration token or null");
    }
    await state.db.setDevicePushToken(device.userId, device.id, token);
    reply.send({ ok: true });
  });
}
