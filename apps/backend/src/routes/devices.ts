import type { FastifyInstance } from "fastify";

import { randomTokenHex, sha256Hex } from "../auth.js";
import { Errors } from "../errors.js";
import type { DeviceKind } from "../models.js";
import { requireAuth } from "../plugins/auth.js";
import type { AppState } from "../state.js";
import { recentlySeen } from "../presence.js";

function parseDeviceKind(kind: unknown): DeviceKind {
  if (kind === "desktop" || kind === "mobile") return kind;
  throw Errors.validation('kind must be "desktop" or "mobile"');
}

function validateDeviceName(name: string): string {
  const trimmed = name.trim();
  if (trimmed.length === 0 || [...trimmed].length > 80) {
    throw Errors.validation("Device name must be 1-80 characters");
  }
  return trimmed;
}

interface RegisterDeviceBody {
  name?: string;
  kind?: string;
  platform?: string;
}

export function registerDeviceRoutes(app: FastifyInstance, state: AppState): void {
  app.post<{ Body: RegisterDeviceBody }>("/v1/devices/register", async (request, reply) => {
    const userId = await requireAuth(state, request, reply);
    if (!userId) return;
    const kind = parseDeviceKind(request.body.kind);
    const name = validateDeviceName(request.body.name ?? "");
    const token = randomTokenHex();
    const device = await state.db.createDevice(crypto.randomUUID(), userId, name, kind, (request.body.platform ?? "").trim(), sha256Hex(token), new Date());
    request.log.info({ userId, deviceId: device.id, kind }, "device registered");
    reply.send({ deviceId: device.id, deviceToken: token });
  });

  app.get("/v1/devices", async (request, reply) => {
    const userId = await requireAuth(state, request, reply);
    if (!userId) return;
    const devices = await state.db.listDevices(userId);
    reply.send({
      // Online: a live socket (mobile-app connection) or a recent request (desktop sync).
      devices: devices.map((d) => ({ ...d, online: state.hub.isOnline(userId, d.id) || recentlySeen(d.lastSeenAt) })),
    });
  });

  app.delete<{ Params: { id: string } }>("/v1/devices/:id", async (request, reply) => {
    const userId = await requireAuth(state, request, reply);
    if (!userId) return;
    const existed = await state.db.deleteDevice(userId, request.params.id);
    if (!existed) throw Errors.deviceNotFound();
    state.hub.forceDisconnect(userId, request.params.id);
    request.log.info({ userId, deviceId: request.params.id }, "device revoked");
    reply.send({});
  });
}
