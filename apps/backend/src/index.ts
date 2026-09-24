import "dotenv/config";

import { buildApp } from "./app.js";
import { loadConfig } from "./config.js";
import { MongoDb } from "./mongoDb.js";
import { createAppState } from "./state.js";

async function main(): Promise<void> {
  const config = loadConfig();
  const db = await MongoDb.connect(config.mongoUri, config.mongoDatabase);
  const state = createAppState(config, db);
  const app = await buildApp(state);

  const shutdown = async () => {
    await app.close();
    await db.close();
    process.exit(0);
  };
  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);

  await app.listen({ port: config.port, host: "0.0.0.0" });
}

main().catch((err) => {
  console.error("job-hunter-backend failed to start:", err);
  process.exit(1);
});
