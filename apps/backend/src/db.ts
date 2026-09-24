import type { DeviceKind, DeviceRecord, UserRecord } from "./models.js";

export interface JsonDoc {
  [key: string]: unknown;
}

/**
 * Everything the backend persists. One implementation talks to MongoDB
 * Atlas (`MongoDb`, the production path -- see `.env.example`); tests use
 * an in-memory fake (`test/inMemoryDb.ts`) so route logic is exercised
 * without a real database, the same way `crates/job-hunter-relay`'s tests
 * used a real (but trivial, file-backed) SQLite database instead of mocking
 * it out -- here a real MongoDB isn't trivial to spin up per-test, so the
 * interface itself is the seam.
 */
export interface Db {
  // Accounts
  createUser(id: string, email: string, passwordHash: string, displayName: string, now: Date): Promise<UserRecord>;
  findUserByEmail(email: string): Promise<{ user: UserRecord; passwordHash: string } | null>;
  findUserById(id: string): Promise<UserRecord | null>;

  // Devices
  createDevice(
    id: string,
    userId: string,
    name: string,
    kind: DeviceKind,
    platform: string,
    tokenHash: string,
    now: Date,
  ): Promise<DeviceRecord>;
  findDeviceByTokenHash(hash: string): Promise<DeviceRecord | null>;
  listDevices(userId: string): Promise<DeviceRecord[]>;
  deleteDevice(userId: string, deviceId: string): Promise<boolean>;
  touchDeviceLastSeen(deviceId: string, now: Date): Promise<void>;

  // Pairing codes (single-use, short TTL)
  createPairingCode(code: string, userId: string, now: Date, expiresAt: Date): Promise<void>;
  /** Consumes the code if valid and unexpired; returns the owning user id, or null. */
  redeemPairingCode(code: string, now: Date): Promise<string | null>;

  // Refresh tokens (single-use, rotated on use)
  insertRefreshToken(hash: string, userId: string, now: Date, expiresAt: Date): Promise<void>;
  /** Consumes the token if valid and unexpired; returns the owning user id, or null. */
  consumeRefreshToken(hash: string, now: Date): Promise<string | null>;

  // Generic domain data -- mirrors crates/job-hunter-core/src/store/mongo.rs's
  // upsert/delete/fetch_all exactly, so the Rust sync layer can swap its
  // MongoDB driver call for an HTTP call to this backend with no change in
  // shape. Every document is scoped to the caller's account.
  upsertDoc(collection: string, userId: string, id: string, value: JsonDoc): Promise<void>;
  deleteDoc(collection: string, userId: string, id: string): Promise<void>;
  fetchAllDocs(collection: string, userId: string): Promise<JsonDoc[]>;
  fetchDoc(collection: string, userId: string, id: string): Promise<JsonDoc | null>;
  /** Documents in `collection` whose `field` equals `value`, scoped to the account. */
  fetchDocsByField(collection: string, userId: string, field: string, value: unknown): Promise<JsonDoc[]>;

  close(): Promise<void>;
}
