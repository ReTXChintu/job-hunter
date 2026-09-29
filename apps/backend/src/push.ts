/**
 * Push notifications to phones through Firebase Cloud Messaging (HTTP v1),
 * so they arrive even when the app is closed. Optional: without a service
 * account configured (JOB_HUNTER_BACKEND_FCM_SERVICE_ACCOUNT) nothing is
 * sent and phones fall back to their periodic check. See docs/deploy.md.
 */
import { createSign } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

import { REPO_ROOT } from "./paths.js";

export interface ServiceAccount {
  project_id: string;
  client_email: string;
  private_key: string;
}

export interface PushMessage {
  title: string;
  body: string;
  /** String values only (FCM data payloads are string maps). */
  data: Record<string, string>;
}

export interface PushTarget {
  deviceId: string;
  token: string;
}

export interface PushSender {
  /** Sends to every target; resolves with the device ids whose token is no
   * longer valid (app uninstalled, token rotated), to be forgotten. */
  send(targets: PushTarget[], message: PushMessage): Promise<string[]>;
}

type Fetch = typeof fetch;

const looksLikePath = (value: string) => /\.json$/i.test(value) || /[\\/]/.test(value);

/**
 * The service account from JSON text, base64 of it, or a path to the JSON
 * file. A relative path is relative to the repository root, like the rest
 * of the .env (PM2 runs the server from apps/backend, so the working
 * directory is not the place to look).
 */
export function parseServiceAccount(raw: string | undefined, baseDir: string = REPO_ROOT): ServiceAccount | null {
  const value = raw?.trim().replace(/^["']|["']$/g, "");
  if (!value) return null;
  let text: string;
  if (value.startsWith("{")) {
    text = value;
  } else if (looksLikePath(value)) {
    const file = path.resolve(baseDir, value);
    if (!existsSync(file)) {
      throw new Error(`JOB_HUNTER_BACKEND_FCM_SERVICE_ACCOUNT points to ${file}, which doesn't exist (relative paths are relative to the repository root).`);
    }
    text = readFileSync(file, "utf8");
  } else {
    text = Buffer.from(value, "base64").toString("utf8");
  }
  let parsed: Partial<ServiceAccount>;
  try {
    parsed = JSON.parse(text) as Partial<ServiceAccount>;
  } catch {
    throw new Error("JOB_HUNTER_BACKEND_FCM_SERVICE_ACCOUNT isn't valid JSON (give the JSON, base64 of it, or a path to the file).");
  }
  if (!parsed.project_id || !parsed.client_email || !parsed.private_key) {
    throw new Error("JOB_HUNTER_BACKEND_FCM_SERVICE_ACCOUNT is JSON but not a Firebase service account (needs project_id, client_email and private_key).");
  }
  return parsed as ServiceAccount;
}

const base64url = (input: Buffer | string) => Buffer.from(input).toString("base64url");

export class FcmSender implements PushSender {
  private accessToken: { value: string; expiresAt: number } | null = null;

  constructor(
    private readonly account: ServiceAccount,
    private readonly fetchImpl: Fetch = fetch,
  ) {}

  /** A Google OAuth access token for FCM, cached until shortly before expiry. */
  private async token(): Promise<string> {
    const now = Math.floor(Date.now() / 1000);
    if (this.accessToken && this.accessToken.expiresAt - 60 > now) return this.accessToken.value;
    const header = base64url(JSON.stringify({ alg: "RS256", typ: "JWT" }));
    const claims = base64url(
      JSON.stringify({
        iss: this.account.client_email,
        scope: "https://www.googleapis.com/auth/firebase.messaging",
        aud: "https://oauth2.googleapis.com/token",
        iat: now,
        exp: now + 3600,
      }),
    );
    const signer = createSign("RSA-SHA256");
    signer.update(`${header}.${claims}`);
    const assertion = `${header}.${claims}.${base64url(signer.sign(this.account.private_key))}`;
    const resp = await this.fetchImpl("https://oauth2.googleapis.com/token", {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({ grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer", assertion }).toString(),
    });
    if (!resp.ok) throw new Error(`Firebase auth failed: ${resp.status} ${await resp.text()}`);
    const body = (await resp.json()) as { access_token: string; expires_in: number };
    this.accessToken = { value: body.access_token, expiresAt: now + body.expires_in };
    return body.access_token;
  }

  async send(targets: PushTarget[], message: PushMessage): Promise<string[]> {
    if (targets.length === 0) return [];
    const token = await this.token();
    const url = `https://fcm.googleapis.com/v1/projects/${this.account.project_id}/messages:send`;
    const invalid: string[] = [];
    await Promise.all(
      targets.map(async (t) => {
        const resp = await this.fetchImpl(url, {
          method: "POST",
          headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
          body: JSON.stringify({
            message: {
              token: t.token,
              notification: { title: message.title, body: message.body },
              data: message.data,
              android: { priority: "high", notification: { channel_id: "job_hunter_alerts" } },
            },
          }),
        });
        if (resp.ok) return;
        const text = await resp.text();
        // The token is gone for good: forget it rather than retry forever.
        if (resp.status === 404 || text.includes("UNREGISTERED") || (resp.status === 400 && text.includes("registration token"))) {
          invalid.push(t.deviceId);
        } else {
          console.warn(`push to device ${t.deviceId} failed: ${resp.status} ${text.slice(0, 200)}`);
        }
      }),
    );
    return invalid;
  }
}

/** The push sender, or null when push isn't configured -- or is configured
 * wrongly: push is optional, so a bad setting is logged and the server runs
 * without it rather than failing to start. */
export function createPushSender(raw: string | undefined, fetchImpl?: Fetch): PushSender | null {
  try {
    const account = parseServiceAccount(raw);
    if (!account) return null;
    console.log(`Push notifications on (Firebase project ${account.project_id}).`);
    return new FcmSender(account, fetchImpl);
  } catch (e) {
    console.error(`Push notifications off: ${(e as Error).message}`);
    return null;
  }
}
