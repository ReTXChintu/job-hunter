/**
 * Pairing codes: the normal way to add a phone. The desktop (already
 * logged in) mints a short-lived, single-use code; the phone redeems it
 * for a device token without ever typing an email or password.
 */
import type { FastifyInstance } from "fastify";

import { randomPairingCode, randomTokenHex, sha256Hex } from "../auth.js";
import { Errors } from "../errors.js";
import { requireAuth } from "../plugins/auth.js";
import type { AppState } from "../state.js";

interface RedeemPairingBody {
  code?: string;
  name?: string;
  platform?: string;
}

function validateDeviceName(name: string): string {
  const trimmed = name.trim();
  if (trimmed.length === 0 || [...trimmed].length > 80) {
    throw Errors.validation("Device name must be 1-80 characters");
  }
  return trimmed;
}

export function registerPairingRoutes(app: FastifyInstance, state: AppState): void {
  app.post("/v1/pairing/create", async (request, reply) => {
    const userId = await requireAuth(state, request, reply);
    if (!userId) return;
    const now = new Date();
    const expiresAt = new Date(now.getTime() + state.config.pairingCodeTtlSecs * 1000);
    // Astronomically unlikely to collide, but retry a couple of times
    // rather than surface an internal error to the user.
    for (let attempt = 0; attempt < 5; attempt++) {
      const code = randomPairingCode();
      try {
        await state.db.createPairingCode(code, userId, now, expiresAt);
        reply.send({ code, expiresAt: expiresAt.toISOString() });
        return;
      } catch (e) {
        if (e instanceof Error && e.message === "CODE_COLLISION") continue;
        throw e;
      }
    }
    throw Errors.internal("could not generate a unique pairing code");
  });

  app.post<{ Body: RedeemPairingBody }>("/v1/pairing/redeem", async (request, reply) => {
    const code = (request.body.code ?? "").trim().toUpperCase();
    const name = validateDeviceName(request.body.name ?? "");
    const userId = await state.db.redeemPairingCode(code, new Date());
    if (!userId) throw Errors.invalidPairingCode();
    const token = randomTokenHex();
    const device = await state.db.createDevice(crypto.randomUUID(), userId, name, "mobile", (request.body.platform ?? "").trim(), sha256Hex(token), new Date());
    request.log.info({ userId, deviceId: device.id }, "device paired via code");
    reply.send({ deviceId: device.id, deviceToken: token, userId });
  });
}
