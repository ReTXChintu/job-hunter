import { config as loadEnvFile } from "dotenv";

import { loadConfig } from "./config.js";
import { MongoDb } from "./mongoDb.js";
import { ROOT_ENV_FILE } from "./paths.js";
import { startServers } from "./server.js";
import { createAppState } from "./state.js";

// The repo has a single `.env`, at its root. Real environment variables
// (PM2, Docker, CI) still win: dotenv never overrides one that's already set.
loadEnvFile({ path: ROOT_ENV_FILE });

async function main(): Promise<void> {
  const config = loadConfig();
  const db = await MongoDb.connect(config.mongoUri, config.mongoDatabase);
  const state = createAppState(config, db);
  const apps = await startServers(state);

  const shutdown = async () => {
    await Promise.all(apps.map((app) => app.close()));
    await db.close();
    process.exit(0);
  };
  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);
}

main().catch((err) => {
  console.error("job-hunter-backend failed to start:", err);
  process.exit(1);
});
