import type { FastifyInstance } from "fastify";

import { buildApp } from "./app.js";
import type { BackendConfig } from "./config.js";
import type { AppState } from "./state.js";

export interface Listener {
  port: number;
  serveWeb: boolean;
  label: "api" | "web" | "api+web";
}

/**
 * Which ports to listen on. With a separate web port, the API port serves
 * only the API (what the desktop and mobile apps use) and the web port serves
 * the web app plus the API calls it makes. Otherwise one port does both.
 */
export function planListeners(config: Pick<BackendConfig, "port" | "webPort">): Listener[] {
  if (config.webPort === null || config.webPort === config.port) {
    return [{ port: config.port, serveWeb: true, label: "api+web" }];
  }
  return [
    { port: config.port, serveWeb: false, label: "api" },
    { port: config.webPort, serveWeb: true, label: "web" },
  ];
}

/**
 * Start every listener over the same state, so a device connected on one
 * port (presence, WebSocket routing) is visible through the other.
 */
export async function startServers(state: AppState, host = "0.0.0.0"): Promise<FastifyInstance[]> {
  const apps: FastifyInstance[] = [];
  try {
    for (const listener of planListeners(state.config)) {
      const app = await buildApp(state, { serveWeb: listener.serveWeb });
      apps.push(app);
      await app.listen({ port: listener.port, host });
      app.log.info({ port: listener.port, serves: listener.label }, "listening");
    }
  } catch (err) {
    await Promise.all(apps.map((a) => a.close()));
    throw err;
  }
  return apps;
}
