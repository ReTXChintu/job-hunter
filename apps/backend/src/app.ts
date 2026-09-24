import cors from "@fastify/cors";
import websocket from "@fastify/websocket";
import Fastify, { type FastifyInstance } from "fastify";

import { BackendError, Errors } from "./errors.js";
import { registerAuthRoutes } from "./routes/auth.js";
import { registerDataRoutes } from "./routes/data.js";
import { registerDeviceRoutes } from "./routes/devices.js";
import { registerPairingRoutes } from "./routes/pairing.js";
import { registerWsRoute } from "./routes/ws.js";
import type { AppState } from "./state.js";

export async function buildApp(state: AppState): Promise<FastifyInstance> {
  const app = Fastify({ logger: true });

  await app.register(cors, {
    origin: state.config.corsOrigins.length > 0 ? state.config.corsOrigins : true,
  });
  await app.register(websocket, {
    errorHandler(error, socket) {
      app.log.error(error, "websocket handler error");
      socket.close();
    },
  });

  app.setErrorHandler((err: Error & { validation?: unknown }, request, reply) => {
    if (err instanceof BackendError) {
      reply.status(err.status).send(err.toBody());
      return;
    }
    if (err.validation) {
      const validationErr = Errors.validation(err.message);
      reply.status(validationErr.status).send(validationErr.toBody());
      return;
    }
    request.log.error(err, "unhandled error");
    const internal = Errors.internal();
    reply.status(internal.status).send(internal.toBody());
  });

  app.get("/healthz", async () => "ok");

  registerAuthRoutes(app, state);
  registerDeviceRoutes(app, state);
  registerPairingRoutes(app, state);
  registerDataRoutes(app, state);
  registerWsRoute(app, state);

  return app;
}
