/**
 * Password hashing, opaque token generation/hashing, and JWT access
 * tokens. Same three-tier design as the Rust relay's `auth.rs`:
 *
 * - **Passwords**: low entropy, user-chosen -> bcrypt (slow, salted).
 * - **Refresh tokens / device tokens**: high entropy, randomly generated
 *   -> SHA-256 is enough (hashing to avoid storing the bearer value
 *   itself, not to resist guessing -- the token itself is already
 *   unguessable).
 * - **Access tokens**: short-lived, stateless -> signed JWT (HS256).
 */
import { randomBytes, createHash } from "node:crypto";
import bcrypt from "bcryptjs";
import { SignJWT, jwtVerify } from "jose";

const BCRYPT_ROUNDS = 12;

export async function hashPassword(password: string): Promise<string> {
  return bcrypt.hash(password, BCRYPT_ROUNDS);
}

export async function verifyPassword(password: string, storedHash: string): Promise<boolean> {
  try {
    return await bcrypt.compare(password, storedHash);
  } catch {
    return false;
  }
}

/** A high-entropy opaque token, hex-encoded: refresh tokens, device tokens, the WebSocket bearer. */
export function randomTokenHex(): string {
  return randomBytes(32).toString("hex");
}

/**
 * A short, human-typeable pairing code. Excludes visually ambiguous
 * characters (0/O, 1/I/L) since it's read off one screen and typed on
 * another.
 */
const PAIRING_ALPHABET = "23456789ABCDEFGHJKMNPQRSTUVWXYZ";
export function randomPairingCode(): string {
  const bytes = randomBytes(6);
  let out = "";
  for (const b of bytes) {
    out += PAIRING_ALPHABET[b % PAIRING_ALPHABET.length];
  }
  return out;
}

export function sha256Hex(input: string): string {
  return createHash("sha256").update(input, "utf8").digest("hex");
}

export async function issueAccessToken(secret: string, userId: string, ttlSecs: number): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  return new SignJWT({})
    .setProtectedHeader({ alg: "HS256" })
    .setSubject(userId)
    .setIssuedAt(now)
    .setExpirationTime(now + ttlSecs)
    .sign(new TextEncoder().encode(secret));
}

/** Returns the user id encoded in a valid, unexpired access token, or null. */
export async function verifyAccessToken(secret: string, token: string): Promise<string | null> {
  try {
    const { payload } = await jwtVerify(token, new TextEncoder().encode(secret), { algorithms: ["HS256"] });
    return typeof payload.sub === "string" ? payload.sub : null;
  } catch {
    return null;
  }
}
