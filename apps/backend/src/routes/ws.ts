/**
 * The one WebSocket endpoint. Desktop and mobile both connect with
 * `?token=<deviceToken>`; everything after the upgrade is routed through
 * the `Hub`. Direct port of `crates/job-hunter-relay/src/routes/ws.rs` --
 * see docs/mobile-protocol.md for the frame format. This file never looks
 * inside a frame's JSON payload, only at its device kind.
 */
import type { FastifyInstance } from "fastify";
import type { WebSocket } from "ws";

import { sha256Hex } from "../auth.js";
import { presenceFrame } from "../hub.js";
import type { AppState } from "../state.js";

const HEARTBEAT_INTERVAL_MS = 20_000;
const IDLE_TIMEOUT_MS = 50_000;

interface WsQuery {
  token?: string;
}

export function registerWsRoute(app: FastifyInstance, state: AppState): void {
  app.get<{ Querystring: WsQuery }>("/v1/ws", { websocket: true }, async (socket, request) => {
    const token = request.query.token ?? "";
    const device = await state.db.findDeviceByTokenHash(sha256Hex(token));
    if (!device) {
      socket.close(4401, "invalid device token");
      return;
    }
    handleSocket(state, socket, device.userId, device.id, device.kind);
  });
}

function handleSocket(state: AppState, socket: WebSocket, userId: string, deviceId: string, kind: "desktop" | "mobile"): void {
  let idleTimer: ReturnType<typeof setTimeout> | undefined;
  const resetIdleTimer = () => {
    if (idleTimer) clearTimeout(idleTimer);
    idleTimer = setTimeout(() => socket.close(), IDLE_TIMEOUT_MS);
  };

  const outcome = state.hub.register(userId, deviceId, kind, {
    send: (text) => {
      if (socket.readyState === socket.OPEN) socket.send(text);
    },
    close: () => socket.close(),
  });
  if (kind === "mobile") {
    socket.send(presenceFrame(outcome.desktopOnline));
  }

  const heartbeat = setInterval(() => {
    if (socket.readyState === socket.OPEN) socket.ping();
  }, HEARTBEAT_INTERVAL_MS);

  resetIdleTimer();

  const cleanup = () => {
    clearInterval(heartbeat);
    if (idleTimer) clearTimeout(idleTimer);
    state.hub.unregister(userId, deviceId, kind);
  };

  socket.on("message", (raw) => {
    resetIdleTimer();
    void state.db.touchDeviceLastSeen(deviceId, new Date());
    const text = raw.toString();
    if (kind === "mobile") {
      if (!state.hub.routeToDesktop(userId, text)) {
        socket.send(offlineResponseFrame(text));
      }
    } else {
      state.hub.routeToMobiles(userId, text);
    }
  });

  socket.on("pong", () => resetIdleTimer());
  socket.on("close", cleanup);
  socket.on("error", cleanup);
}

/**
 * Echoes back the request's own `id` (falling back to a fresh one if the
 * inbound frame was not valid JSON) so the mobile client's pending-request
 * map resolves the same way it would for a real desktop reply.
 */
function offlineResponseFrame(inboundText: string): string {
  let id: unknown = crypto.randomUUID();
  try {
    const parsed = JSON.parse(inboundText) as { id?: unknown };
    if (parsed && "id" in parsed) id = parsed.id;
  } catch {
    // not valid JSON; keep the fresh id
  }
  return JSON.stringify({
    v: 1,
    id,
    type: "response",
    payload: { ok: false, error: { code: "DESKTOP_OFFLINE", message: "Your desktop is not online right now." } },
  });
}
