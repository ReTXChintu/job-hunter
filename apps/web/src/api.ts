/**
 * Client for the Job Hunter backend. The web app is served by the backend
 * itself (apps/backend/src/routes/static.ts), so every path is relative to
 * the page's own origin -- there is no server URL to configure.
 *
 * Reads use the backend's REST read views. Actions never write data
 * directly: they go through `POST /v1/desktop/request`, which relays them to
 * the user's desktop, where the same approval-gated code as the desktop's
 * own buttons runs them (crates/job-hunter-core/src/remote/dispatch.rs).
 *
 * Signing in uses the same email/password account as the desktop and mobile
 * apps. The short-lived access token is refreshed transparently with the
 * refresh token; if that fails too, the session ends.
 */
import type {
  AgentState,
  AnswerRecord,
  Application,
  ApplicationListItem,
  ApplicationStatus,
  Job,
  JobAnalysis,
  Notification,
  PlatformProfileView,
  RunKind,
} from "@job-hunter/types";

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
  version: string | null;
  build: number | null;
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

/** A question the desktop asked, with the answer the user typed. */
export interface QuestionAnswer {
  question: string;
  answer: string;
}

export interface AgentStatusSummary {
  state: AgentState;
  paused: boolean;
  runKind: RunKind | null;
  progress: number | null;
}

/**
 * What to tell the user when the desktop can't do something. The relay's
 * own failures get a plain explanation; errors the desktop raised already
 * carry a user-facing message, which is kept.
 */
const DESKTOP_ERROR_MESSAGES: Record<string, string> = {
  DESKTOP_OFFLINE: "Your desktop is offline. Open Job Hunter on your computer and try again.",
  DESKTOP_TIMEOUT: "Your desktop didn't answer in time. Make sure Job Hunter is running on your computer and try again.",
  AGENT_BUSY: "The agent on your desktop is busy with another task. Try again once it finishes.",
  INVALID_TRANSITION: "This application changed since the page loaded, so that's no longer possible. Refresh to see where it stands.",
  NOT_FOUND: "Your desktop couldn't find this item. It may have been removed.",
  NETWORK: "Couldn't reach the server. Check your connection and try again.",
};

export function desktopErrorMessage(code: string, fallback: string): string {
  return DESKTOP_ERROR_MESSAGES[code] ?? (fallback.trim() || "Your desktop couldn't do that.");
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

  login(email: string, password: string): Promise<Session> {
    return this.authenticate("/v1/auth/login", email, password);
  }

  /** Creates the account and signs in. The desktop and mobile apps sign in to the same account. */
  register(email: string, password: string): Promise<Session> {
    return this.authenticate("/v1/auth/register", email, password);
  }

  private async authenticate(path: string, email: string, password: string): Promise<Session> {
    const resp = await this.fetchImpl(`${this.base}${path}`, {
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

  /** An authenticated request; a 401 refreshes the access token once and retries. */
  private async authed<T>(path: string, body?: unknown): Promise<T> {
    const send = () => {
      const headers: Record<string, string> = this.session ? { Authorization: `Bearer ${this.session.accessToken}` } : {};
      if (body === undefined) return this.fetchImpl(`${this.base}${path}`, { headers });
      headers["Content-Type"] = "application/json";
      return this.fetchImpl(`${this.base}${path}`, { method: "POST", headers, body: JSON.stringify(body) });
    };
    let resp = await send();
    if (resp.status === 401 && this.session && (await this.refresh())) {
      resp = await send();
    }
    if (resp.status === 401) this.setSession(null);
    return parse<T>(resp);
  }

  get<T>(path: string): Promise<T> {
    return this.authed<T>(path);
  }

  /**
   * Asks the user's desktop to do something (see dispatch.rs for the
   * request types). Resolves only once the desktop has done it and replied;
   * rejects with an `ApiError` whose message is meant for the user.
   */
  async desktop<T = unknown>(type: string, payload: Record<string, unknown> = {}): Promise<T> {
    let reply: { data: T };
    try {
      reply = await this.authed<{ data: T }>("/v1/desktop/request", { type, payload });
    } catch (err) {
      if (err instanceof ApiError) throw new ApiError(desktopErrorMessage(err.code, err.message), err.code, err.status);
      throw new ApiError(desktopErrorMessage("NETWORK", ""), "NETWORK", 0);
    }
    return reply.data;
  }

  // ---- actions, run by the desktop ----------------------------------------------

  /** Answers an application's pending questions; the desktop continues the application. */
  answerApplicationQuestions(id: string, answers: QuestionAnswer[]): Promise<Application> {
    return this.desktop("answer_application_questions", { id, answers });
  }

  /** Approves an application; the desktop starts applying straight away. */
  approveApplication(id: string): Promise<Application> {
    return this.desktop("approve_application", { id });
  }

  rejectApplication(id: string, reason: string): Promise<Application> {
    return this.desktop("reject_application", { id, reason });
  }

  /** Runs (or retries) an approved application on the desktop. */
  applyApplication(id: string): Promise<{ runId: string }> {
    return this.desktop("apply_application", { id });
  }

  markApplicationApplied(id: string, note = ""): Promise<Application> {
    return this.desktop("mark_manual_application_complete", { id, note });
  }

  setApplicationStatus(id: string, status: ApplicationStatus, note = ""): Promise<Application> {
    return this.desktop("set_application_status", { id, status, note });
  }

  startJobHunt(options: { sources?: string[]; discoverOnly?: boolean } = {}): Promise<{ runId: string }> {
    return this.desktop("start_job_hunt", options);
  }

  stopJobHunt(): Promise<{ stopped: boolean }> {
    return this.desktop("stop_job_hunt");
  }

  /** Reads Gmail (inbox and spam) for employers' replies. */
  checkInbox(): Promise<{ runId: string }> {
    return this.desktop("check_inbox");
  }

  generateResume(jobId: string): Promise<{ runId: string }> {
    return this.desktop("generate_resume", { jobId });
  }

  rejectJob(id: string): Promise<Job> {
    return this.desktop("reject_job", { id });
  }

  agentStatus(): Promise<AgentStatusSummary> {
    return this.desktop("get_agent_status");
  }

  savedAnswers(): Promise<AnswerRecord[]> {
    return this.desktop("list_answers");
  }

  saveAnswer(answer: { id?: string; question: string; answer: string }): Promise<AnswerRecord> {
    return this.desktop("save_answer", answer);
  }

  platformProfiles(): Promise<PlatformProfileView[]> {
    return this.desktop("list_platform_profiles");
  }

  syncPlatformProfile(platform: string, resume = false): Promise<{ runId: string }> {
    return this.desktop("sync_platform_profile", { platform, resume });
  }

  answerPlatformQuestions(platform: string, answers: QuestionAnswer[]): Promise<{ runId: string }> {
    return this.desktop("answer_platform_questions", { platform, answers });
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

  /** Notifications the desktop synced (finished hunts, things that need you, failures). */
  async notifications(): Promise<Notification[]> {
    return (await this.get<{ documents: Notification[] }>("/v1/data/notifications")).documents;
  }

  async analyses(): Promise<JobAnalysis[]> {
    return (await this.get<{ documents: JobAnalysis[] }>("/v1/data/job_analyses")).documents;
  }

  /** Public: works signed out too, for the login page's download buttons. */
  downloads(): Promise<{ android: DownloadInfo | null; windows: DownloadInfo | null }> {
    return this.get("/v1/downloads");
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
