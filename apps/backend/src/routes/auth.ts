import type { FastifyInstance } from "fastify";

import { hashPassword, issueAccessToken, randomTokenHex, sha256Hex, verifyPassword } from "../auth.js";
import { Errors } from "../errors.js";
import type { AppState } from "../state.js";

export function normalizeEmail(email: string): string {
  const trimmed = email.trim().toLowerCase();
  if (!trimmed.includes("@") || trimmed.length < 5 || trimmed.length > 254) {
    throw Errors.validation("Enter a valid email address");
  }
  return trimmed;
}

export function validatePassword(password: string): void {
  if ([...password].length < 8) {
    throw Errors.validation("Password must be at least 8 characters");
  }
}

interface RegisterBody {
  email?: string;
  password?: string;
  displayName?: string;
}
interface LoginBody {
  email?: string;
  password?: string;
}
interface RefreshBody {
  refreshToken?: string;
}

async function issueSession(state: AppState, userId: string) {
  const now = new Date();
  const accessToken = await issueAccessToken(state.config.jwtSecret, userId, state.config.accessTokenTtlSecs);
  const refreshToken = randomTokenHex();
  const expiresAt = new Date(now.getTime() + state.config.refreshTokenTtlDays * 24 * 60 * 60 * 1000);
  await state.db.insertRefreshToken(sha256Hex(refreshToken), userId, now, expiresAt);
  return { userId, accessToken, refreshToken };
}

function rateLimitKey(request: { ip: string; headers: Record<string, unknown> }, email: string): string {
  const forwarded = request.headers["x-forwarded-for"];
  const ip = (typeof forwarded === "string" ? forwarded.split(",")[0]?.trim() : undefined) || request.ip;
  return `${ip}:${email}`;
}

export function registerAuthRoutes(app: FastifyInstance, state: AppState): void {
  app.post<{ Body: RegisterBody }>("/v1/auth/register", async (request, reply) => {
    const email = normalizeEmail(request.body.email ?? "");
    if (!state.authLimiter.check(rateLimitKey(request, email))) throw Errors.rateLimited();
    validatePassword(request.body.password ?? "");
    const displayName = request.body.displayName?.trim() || email.split("@")[0] || email;
    const passwordHash = await hashPassword(request.body.password!);
    let user;
    try {
      user = await state.db.createUser(crypto.randomUUID(), email, passwordHash, displayName, new Date());
    } catch (e) {
      if (e instanceof Error && e.message === "EMAIL_TAKEN") throw Errors.emailTaken();
      throw e;
    }
    request.log.info({ userId: user.id }, "account registered");
    reply.send(await issueSession(state, user.id));
  });

  app.post<{ Body: LoginBody }>("/v1/auth/login", async (request, reply) => {
    const email = normalizeEmail(request.body.email ?? "");
    if (!state.authLimiter.check(rateLimitKey(request, email))) throw Errors.rateLimited();
    const found = await state.db.findUserByEmail(email);
    if (!found || !(await verifyPassword(request.body.password ?? "", found.passwordHash))) {
      throw Errors.invalidCredentials();
    }
    request.log.info({ userId: found.user.id }, "login succeeded");
    reply.send(await issueSession(state, found.user.id));
  });

  app.post<{ Body: RefreshBody }>("/v1/auth/refresh", async (request, reply) => {
    const refreshToken = request.body.refreshToken ?? "";
    const now = new Date();
    const userId = await state.db.consumeRefreshToken(sha256Hex(refreshToken), now);
    if (!userId) throw Errors.invalidRefreshToken();
    const accessToken = await issueAccessToken(state.config.jwtSecret, userId, state.config.accessTokenTtlSecs);
    const newRefreshToken = randomTokenHex();
    const expiresAt = new Date(now.getTime() + state.config.refreshTokenTtlDays * 24 * 60 * 60 * 1000);
    await state.db.insertRefreshToken(sha256Hex(newRefreshToken), userId, now, expiresAt);
    reply.send({ accessToken, refreshToken: newRefreshToken });
  });

  app.post<{ Body: RefreshBody }>("/v1/auth/logout", async (request, reply) => {
    // Consuming (rather than merely looking up) is enough to invalidate it;
    // we don't need to know whether it was valid to report success.
    await state.db.consumeRefreshToken(sha256Hex(request.body.refreshToken ?? ""), new Date());
    reply.send({});
  });
}
