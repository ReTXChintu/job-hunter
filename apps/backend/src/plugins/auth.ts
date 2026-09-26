/**
 * Resolves the authenticated user id from an `Authorization: Bearer <token>`
 * header, accepting either a short-lived JWT access token or a long-lived
 * device token -- same dual acceptance as
 * `crates/job-hunter-relay/src/extractor.rs`, and for the same reason:
 * neither app holds on to a password or refresh token past its first
 * login, so every authenticated endpoint after device registration must
 * also accept the device token.
 */
import type { FastifyReply, FastifyRequest } from "fastify";

import { sha256Hex, verifyAccessToken } from "../auth.js";
import { Errors } from "../errors.js";
import { markSeen } from "../presence.js";
import type { AppState } from "../state.js";

export async function requireAuth(state: AppState, request: FastifyRequest, reply: FastifyReply): Promise<string | undefined> {
  const header = request.headers.authorization;
  const token = header?.startsWith("Bearer ") ? header.slice("Bearer ".length) : undefined;
  if (!token) {
    const err = Errors.unauthorized();
    reply.status(err.status).send(err.toBody());
    return undefined;
  }
  const fromJwt = await verifyAccessToken(state.config.jwtSecret, token);
  if (fromJwt) return fromJwt;
  const device = await state.db.findDeviceByTokenHash(sha256Hex(token));
  if (device) {
    markSeen(state, device.id);
    return device.userId;
  }
  const err = Errors.unauthorized();
  reply.status(err.status).send(err.toBody());
  return undefined;
}
