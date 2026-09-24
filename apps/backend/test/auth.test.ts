import { describe, expect, it } from "vitest";

import { hashPassword, issueAccessToken, randomPairingCode, randomTokenHex, sha256Hex, verifyAccessToken, verifyPassword } from "../src/auth.js";

describe("password hashing", () => {
  it("round-trips and rejects a wrong password", async () => {
    const hash = await hashPassword("correct horse battery staple");
    expect(await verifyPassword("correct horse battery staple", hash)).toBe(true);
    expect(await verifyPassword("wrong password", hash)).toBe(false);
    expect(hash).not.toContain("correct horse");
  });
});

describe("random tokens and codes", () => {
  it("are unique and well-formed", () => {
    const a = randomTokenHex();
    const b = randomTokenHex();
    expect(a).not.toBe(b);
    expect(a).toHaveLength(64);
    expect(a).toMatch(/^[0-9a-f]+$/);

    const code = randomPairingCode();
    expect(code).toHaveLength(6);
    expect(code).toMatch(/^[A-Z0-9]+$/);
    expect(code).not.toMatch(/[01OIL]/);
  });
});

describe("sha256Hex", () => {
  it("is deterministic", () => {
    expect(sha256Hex("abc")).toBe(sha256Hex("abc"));
    expect(sha256Hex("abc")).not.toBe(sha256Hex("abd"));
  });
});

describe("access tokens", () => {
  it("round-trip and reject tampering and expiry", async () => {
    const token = await issueAccessToken("secret", "user-1", 60);
    expect(await verifyAccessToken("secret", token)).toBe("user-1");
    expect(await verifyAccessToken("different-secret", token)).toBeNull();

    const expired = await issueAccessToken("secret", "user-1", -120);
    expect(await verifyAccessToken("secret", expired)).toBeNull();
  });

  it("rejects a garbage token", async () => {
    expect(await verifyAccessToken("secret", "not-a-jwt")).toBeNull();
  });
});
