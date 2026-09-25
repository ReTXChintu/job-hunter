/**
 * Read-only client for the Job Hunter backend. The web app is served by the
 * backend itself (apps/backend/src/routes/static.ts), so every path is
 * relative to the page's own origin -- there is no server URL to configure.
 *
 * Signing in uses the same email/password account as the desktop and mobile
 * apps. The short-lived access token is refreshed transparently with the
 * refresh token; if that fails too, the session ends.
 */
import type { ApplicationListItem, Job, JobAnalysis } from "@job-hunter/types";

export interface Session {
  email: string;
  accessToken: string;
  refreshToken: string;
}

export interface Device {
  id: string;
  name: string;
  kind: "desktop" | "mobile";
  platform: string;
  createdAt: string;
  lastSeenAt: string | null;
  online: boolean;
}

export interface DownloadInfo {
  fileName: string;
  sizeBytes: number;
  updatedAt: string;
  url: string;
}

export class ApiError extends Error {
  constructor(
    message: string,
    readonly code: string,
    readonly status: number,
  ) {
    super(message);
  }
}

type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

async function parse<T>(resp: Response): Promise<T> {
  const text = await resp.text();
  const body: unknown = text ? JSON.parse(text) : {};
  if (!resp.ok) {
    const err = (body as { error?: { code?: string; message?: string } }).error;
    throw new ApiError(err?.message ?? `Request failed (${resp.status})`, err?.code ?? "HTTP_ERROR", resp.status);
  }
  return body as T;
}

export class ApiClient {
  private session: Session | null;
  private refreshing: Promise<boolean> | null = null;

  constructor(
    session: Session | null,
    private readonly onSessionChange: (session: Session | null) => void,
    private readonly fetchImpl: FetchLike = (input, init) => fetch(input, init),
    private readonly base = "",
  ) {
    this.session = session;
  }

  get current(): Session | null {
    return this.session;
  }

  private setSession(session: Session | null): void {
    this.session = session;
    this.onSessionChange(session);
  }

  async login(email: string, password: string): Promise<Session> {
    const resp = await this.fetchImpl(`${this.base}/v1/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, password }),
    });
    const tokens = await parse<{ accessToken: string; refreshToken: string }>(resp);
    const session = { email: email.trim().toLowerCase(), accessToken: tokens.accessToken, refreshToken: tokens.refreshToken };
    this.setSession(session);
    return session;
  }

  async logout(): Promise<void> {
    const refreshToken = this.session?.refreshToken;
    this.setSession(null);
    if (refreshToken) {
      await this.fetchImpl(`${this.base}/v1/auth/logout`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ refreshToken }),
      }).catch(() => undefined);
    }
  }

  /** One refresh at a time, however many requests hit a 401 together. */
  private refresh(): Promise<boolean> {
    if (!this.refreshing) {
      this.refreshing = (async () => {
        const current = this.session;
        if (!current) return false;
        try {
          const resp = await this.fetchImpl(`${this.base}/v1/auth/refresh`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ refreshToken: current.refreshToken }),
          });
          const tokens = await parse<{ accessToken: string; refreshToken: string }>(resp);
          this.setSession({ ...current, accessToken: tokens.accessToken, refreshToken: tokens.refreshToken });
          return true;
        } catch {
          this.setSession(null);
          return false;
        }
      })().finally(() => {
        this.refreshing = null;
      });
    }
    return this.refreshing;
  }

  async get<T>(path: string): Promise<T> {
    const send = () =>
      this.fetchImpl(`${this.base}${path}`, {
        headers: this.session ? { Authorization: `Bearer ${this.session.accessToken}` } : {},
      });
    let resp = await send();
    if (resp.status === 401 && this.session && (await this.refresh())) {
      resp = await send();
    }
    if (resp.status === 401) this.setSession(null);
    return parse<T>(resp);
  }

  applications(): Promise<ApplicationListItem[]> {
    return this.get("/v1/applications");
  }

  application(id: string): Promise<ApplicationListItem> {
    return this.get(`/v1/applications/${encodeURIComponent(id)}`);
  }

  async devices(): Promise<Device[]> {
    return (await this.get<{ devices: Device[] }>("/v1/devices")).devices;
  }

  async jobs(): Promise<Job[]> {
    return (await this.get<{ documents: Job[] }>("/v1/data/jobs")).documents;
  }

  async analyses(): Promise<JobAnalysis[]> {
    return (await this.get<{ documents: JobAnalysis[] }>("/v1/data/job_analyses")).documents;
  }

  async androidDownload(): Promise<DownloadInfo | null> {
    return (await this.get<{ android: DownloadInfo | null }>("/v1/downloads")).android;
  }
}

const STORAGE_KEY = "job-hunter.session";

export function loadSession(): Session | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const s = JSON.parse(raw) as Partial<Session>;
    return s.accessToken && s.refreshToken && s.email ? (s as Session) : null;
  } catch {
    return null;
  }
}

export function storeSession(session: Session | null): void {
  try {
    if (session) localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
    else localStorage.removeItem(STORAGE_KEY);
  } catch {
    // Storage unavailable (private window): the session just won't survive a reload.
  }
}
