import type { BackendConfig } from "./config.js";
import type { Db } from "./db.js";
import { Hub } from "./hub.js";
import { createPushSender, type PushSender } from "./push.js";
import { RateLimiter } from "./rateLimit.js";

export interface AppState {
  config: BackendConfig;
  db: Db;
  hub: Hub;
  authLimiter: RateLimiter;
  /** deviceId -> when its "last seen" was last written (see presence.ts). */
  lastTouched: Map<string, number>;
  /** Firebase push to phones, when configured. */
  push: PushSender | null;
}

export function createAppState(config: BackendConfig, db: Db, push?: PushSender | null): AppState {
  return {
    config,
    db,
    hub: new Hub(),
    authLimiter: new RateLimiter(10, 60_000),
    lastTouched: new Map(),
    push: push === undefined ? createPushSender(config.fcmServiceAccount) : push,
  };
}
