import path from "node:path";
import { fileURLToPath } from "node:url";

/**
 * The repository root. This file sits at `apps/backend/src/paths.ts` in
 * development and `apps/backend/dist/paths.js` when built, so the root is
 * three levels up either way, regardless of the directory the server was
 * started from.
 */
export const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");

/** The one `.env` for the whole repo; see `.env.example` at the root. */
export const ROOT_ENV_FILE = path.join(REPO_ROOT, ".env");
