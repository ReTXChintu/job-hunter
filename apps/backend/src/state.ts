import type { BackendConfig } from "./config.js";
import type { Db } from "./db.js";
import { Hub } from "./hub.js";
import { RateLimiter } from "./rateLimit.js";

export interface AppState {
  config: BackendConfig;
  db: Db;
  hub: Hub;
  authLimiter: RateLimiter;
}

export function createAppState(config: BackendConfig, db: Db): AppState {
  return {
    config,
    db,
    hub: new Hub(),
    authLimiter: new RateLimiter(10, 60_000),
  };
}
