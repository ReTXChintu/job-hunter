/**
 * PM2 process file for the self-hosted backend (see docs/deploy.md).
 *
 * The server's configuration, including its port, lives in the repo's single
 * root .env -- the backend loads it itself on start (it locates the repo
 * root from its own file, not the working directory), so nothing here
 * duplicates it.
 *
 * Use the root package.json scripts rather than calling pm2 directly:
 *   pnpm server:build     build the web app and the backend
 *   pnpm server:start     start (or reload, if already running)
 *   pnpm server:stop | server:restart | server:logs | server:status
 */
module.exports = {
  apps: [
    {
      name: "job-hunter-backend",
      cwd: __dirname + "/apps/backend",
      script: "dist/index.js",
      instances: 1, // presence and WebSocket routing are in-memory: never cluster this
      exec_mode: "fork",
      autorestart: true,
      max_restarts: 20,
      restart_delay: 3000,
      max_memory_restart: "400M",
      time: true,
      env: {
        NODE_ENV: "production",
      },
    },
  ],
};
