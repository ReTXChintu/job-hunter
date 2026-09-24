import { describe, expect, it } from "vitest";

import { RateLimiter } from "../src/rateLimit.js";

describe("RateLimiter", () => {
  it("allows up to the limit then blocks, per key", () => {
    const limiter = new RateLimiter(3, 60_000);
    expect(limiter.check("1.2.3.4")).toBe(true);
    expect(limiter.check("1.2.3.4")).toBe(true);
    expect(limiter.check("1.2.3.4")).toBe(true);
    expect(limiter.check("1.2.3.4")).toBe(false);
    expect(limiter.check("5.6.7.8")).toBe(true);
  });
});
