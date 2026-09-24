import type { Db, JsonDoc } from "../src/db.js";
import type { DeviceKind, DeviceRecord, UserRecord } from "../src/models.js";

interface StoredUser extends UserRecord {
  passwordHash: string;
}

interface StoredPairingCode {
  userId: string;
  expiresAt: Date;
  redeemedAt: Date | null;
}

interface StoredRefreshToken {
  userId: string;
  expiresAt: Date;
  consumedAt: Date | null;
}

/**
 * A pure in-memory stand-in for `MongoDb`, used by every route test so they
 * exercise real request/response logic without a real database. It's not a
 * mock of individual calls -- it's a second, real (if naive) implementation
 * of the same `Db` interface, so a test failure means the route logic is
 * actually wrong, not that a mock was set up incorrectly.
 */
export class InMemoryDb implements Db {
  private users = new Map<string, StoredUser>();
  private usersByEmail = new Map<string, string>();
  private devices = new Map<string, DeviceRecord>();
  private devicesByTokenHash = new Map<string, string>();
  private pairingCodes = new Map<string, StoredPairingCode>();
  private refreshTokens = new Map<string, StoredRefreshToken>();
  private domainDocs = new Map<string, Map<string, JsonDoc>>();

  async createUser(id: string, email: string, passwordHash: string, displayName: string, now: Date): Promise<UserRecord> {
    if (this.usersByEmail.has(email)) throw new Error("EMAIL_TAKEN");
    const record: StoredUser = { id, email, displayName, createdAt: now.toISOString(), passwordHash };
    this.users.set(id, record);
    this.usersByEmail.set(email, id);
    return { id, email, displayName, createdAt: record.createdAt };
  }

  async findUserByEmail(email: string) {
    const id = this.usersByEmail.get(email);
    if (!id) return null;
    const stored = this.users.get(id)!;
    return { user: { id: stored.id, email: stored.email, displayName: stored.displayName, createdAt: stored.createdAt }, passwordHash: stored.passwordHash };
  }

  async findUserById(id: string): Promise<UserRecord | null> {
    const stored = this.users.get(id);
    if (!stored) return null;
    return { id: stored.id, email: stored.email, displayName: stored.displayName, createdAt: stored.createdAt };
  }

  async createDevice(id: string, userId: string, name: string, kind: DeviceKind, platform: string, tokenHash: string, now: Date): Promise<DeviceRecord> {
    const record: DeviceRecord = { id, userId, name, kind, platform, createdAt: now.toISOString(), lastSeenAt: null };
    this.devices.set(id, record);
    this.devicesByTokenHash.set(tokenHash, id);
    return record;
  }

  async findDeviceByTokenHash(hash: string): Promise<DeviceRecord | null> {
    const id = this.devicesByTokenHash.get(hash);
    return id ? (this.devices.get(id) ?? null) : null;
  }

  async listDevices(userId: string): Promise<DeviceRecord[]> {
    return [...this.devices.values()].filter((d) => d.userId === userId);
  }

  async deleteDevice(userId: string, deviceId: string): Promise<boolean> {
    const d = this.devices.get(deviceId);
    if (!d || d.userId !== userId) return false;
    this.devices.delete(deviceId);
    for (const [hash, id] of this.devicesByTokenHash) {
      if (id === deviceId) this.devicesByTokenHash.delete(hash);
    }
    return true;
  }

  async touchDeviceLastSeen(deviceId: string, now: Date): Promise<void> {
    const d = this.devices.get(deviceId);
    if (d) d.lastSeenAt = now.toISOString();
  }

  async createPairingCode(code: string, userId: string, now: Date, expiresAt: Date): Promise<void> {
    void now;
    this.pairingCodes.set(code, { userId, expiresAt, redeemedAt: null });
  }

  async redeemPairingCode(code: string, now: Date): Promise<string | null> {
    const entry = this.pairingCodes.get(code);
    if (!entry || entry.redeemedAt || entry.expiresAt <= now) return null;
    entry.redeemedAt = now;
    return entry.userId;
  }

  async insertRefreshToken(hash: string, userId: string, now: Date, expiresAt: Date): Promise<void> {
    void now;
    this.refreshTokens.set(hash, { userId, expiresAt, consumedAt: null });
  }

  async consumeRefreshToken(hash: string, now: Date): Promise<string | null> {
    const entry = this.refreshTokens.get(hash);
    if (!entry || entry.consumedAt || entry.expiresAt <= now) return null;
    entry.consumedAt = now;
    return entry.userId;
  }

  private collection(name: string): Map<string, JsonDoc> {
    let c = this.domainDocs.get(name);
    if (!c) {
      c = new Map();
      this.domainDocs.set(name, c);
    }
    return c;
  }

  async upsertDoc(collection: string, userId: string, id: string, value: JsonDoc): Promise<void> {
    this.collection(collection).set(scopedKey(userId, id), { ...value, id, userId });
  }

  async deleteDoc(collection: string, userId: string, id: string): Promise<void> {
    this.collection(collection).delete(scopedKey(userId, id));
  }

  async fetchAllDocs(collection: string, userId: string): Promise<JsonDoc[]> {
    return [...this.collection(collection).values()].filter((d) => d.userId === userId);
  }

  async fetchDoc(collection: string, userId: string, id: string): Promise<JsonDoc | null> {
    return this.collection(collection).get(scopedKey(userId, id)) ?? null;
  }

  async fetchDocsByField(collection: string, userId: string, field: string, value: unknown): Promise<JsonDoc[]> {
    return [...this.collection(collection).values()].filter((d) => d.userId === userId && d[field] === value);
  }

  async close(): Promise<void> {
    // nothing to release
  }
}

function scopedKey(userId: string, id: string): string {
  return `${userId}:${id}`;
}
