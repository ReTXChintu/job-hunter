/**
 * A small in-memory sliding-window rate limiter for the auth endpoints.
 * State is not persisted across restarts, matching
 * `crates/job-hunter-relay/src/rate_limit.rs` -- a restart is itself a
 * full reset of any in-progress brute-force attempt, which is an
 * acceptable trade-off for a personal-use backend.
 */
export class RateLimiter {
  private readonly hits = new Map<string, number[]>();

  constructor(
    private readonly maxAttempts: number,
    private readonly windowMs: number,
  ) {}

  /** Records an attempt for `key`; returns true if it should be allowed. */
  check(key: string): boolean {
    const now = Date.now();
    const existing = this.hits.get(key) ?? [];
    const recent = existing.filter((t) => now - t < this.windowMs);
    if (recent.length >= this.maxAttempts) {
      this.hits.set(key, recent);
      return false;
    }
    recent.push(now);
    this.hits.set(key, recent);
    return true;
  }
}
