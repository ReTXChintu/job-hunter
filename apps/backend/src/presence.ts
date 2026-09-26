/**
 * "Is this device online?" for devices that don't hold a WebSocket open.
 *
 * The mobile-app connection keeps a socket open, which the Hub tracks. The
 * desktop's data sync doesn't: it makes plain HTTP requests, plus a
 * heartbeat every minute while idle (crates/job-hunter-core/src/store/sync.rs).
 * Any request authenticated with a device token counts as "seen", and a
 * device seen recently counts as online.
 */
import type { AppState } from "./state.js";

/** Seen within this long counts as online: two missed one-minute heartbeats. */
export const PRESENCE_WINDOW_MS = 150_000;

/** Don't write "last seen" to the database more often than this per device. */
const TOUCH_EVERY_MS = 30_000;

export function markSeen(state: AppState, deviceId: string, now = Date.now()): void {
  const last = state.lastTouched.get(deviceId) ?? 0;
  if (now - last < TOUCH_EVERY_MS) return;
  state.lastTouched.set(deviceId, now);
  // Presence is best-effort: a failed write must never fail the request.
  state.db.touchDeviceLastSeen(deviceId, new Date(now)).catch(() => state.lastTouched.delete(deviceId));
}

export function recentlySeen(lastSeenAt: string | null, now = Date.now()): boolean {
  if (!lastSeenAt) return false;
  const seen = Date.parse(lastSeenAt);
  return Number.isFinite(seen) && now - seen <= PRESENCE_WINDOW_MS;
}
