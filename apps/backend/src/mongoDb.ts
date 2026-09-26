import { MongoClient, type Collection, type Db as MongoDatabase } from "mongodb";

import type { Db, JsonDoc } from "./db.js";
import { DOMAIN_COLLECTIONS, type DeviceKind, type DeviceRecord, type UserRecord } from "./models.js";

// Internal collections, prefixed with `_` so they never collide with a
// domain collection name (see models.ts's DOMAIN_COLLECTIONS) even though
// they live in the same Atlas database.
const USERS = "_accounts";
const DEVICES = "_devices";
const PAIRING_CODES = "_pairingCodes";
const REFRESH_TOKENS = "_refreshTokens";

interface UserDoc {
  _id: string;
  email: string;
  passwordHash: string;
  displayName: string;
  createdAt: string;
}

interface DeviceDoc {
  _id: string;
  userId: string;
  name: string;
  kind: DeviceKind;
  platform: string;
  tokenHash: string;
  createdAt: string;
  lastSeenAt: string | null;
}

interface PairingCodeDoc {
  _id: string; // the code itself
  userId: string;
  expiresAt: string;
  redeemedAt: string | null;
}

interface RefreshTokenDoc {
  _id: string; // sha256 hash of the token
  userId: string;
  expiresAt: string;
  consumedAt: string | null;
}

interface DomainDoc extends JsonDoc {
  /** `<userId>:<id>`, see {@link scopedId}. */
  _id: string;
  userId: string;
  id: string;
}

/**
 * The Mongo key for a domain document. Scoped by account because ids are
 * only unique within one account: every desktop's user record, for one, is
 * "local-user". Keying by the bare id made a second account's write collide
 * with the first account's document (duplicate key -> 500).
 */
export function scopedId(userId: string, id: string): string {
  return `${userId}:${id}`;
}

export class MongoDb implements Db {
  private constructor(
    private readonly client: MongoClient,
    private readonly database: MongoDatabase,
  ) {}

  static async connect(uri: string, databaseName: string): Promise<MongoDb> {
    const client = new MongoClient(uri, { appName: "JobHunterBackend" });
    await client.connect();
    const database = client.db(databaseName);
    await database.command({ ping: 1 });
    const db = new MongoDb(client, database);
    await db.ensureIndexes();
    const migrated = await db.migrateLegacyDomainIds();
    if (migrated > 0) console.info(`Re-keyed ${migrated} document(s) to per-account ids.`);
    return db;
  }

  /**
   * One-time fix for documents written before ids were account-scoped:
   * re-insert each under `scopedId(userId, id)` and drop the old copy.
   * Idempotent; a no-op once nothing is left to move.
   */
  async migrateLegacyDomainIds(): Promise<number> {
    let moved = 0;
    for (const name of DOMAIN_COLLECTIONS) {
      const collection = this.domain(name);
      const docs = await collection.find({}).toArray();
      for (const doc of docs) {
        if (typeof doc.userId !== "string" || doc._id.startsWith(`${doc.userId}:`)) continue;
        const id = typeof doc.id === "string" && doc.id ? doc.id : doc._id;
        const rekeyed: DomainDoc = { ...doc, id, _id: scopedId(doc.userId, id) };
        try {
          await collection.insertOne(rekeyed);
        } catch (err) {
          // Already re-keyed by an earlier, interrupted run: keep that copy.
          if (!isDuplicateKeyError(err)) throw err;
        }
        await collection.deleteOne({ _id: doc._id });
        moved += 1;
      }
    }
    return moved;
  }

  private async ensureIndexes(): Promise<void> {
    await this.users().createIndex({ email: 1 }, { unique: true });
    await this.devices().createIndex({ tokenHash: 1 }, { unique: true });
    await this.devices().createIndex({ userId: 1 });
    await this.pairingCodes().createIndex({ expiresAt: 1 }, { expireAfterSeconds: 0 });
    await this.refreshTokens().createIndex({ expiresAt: 1 }, { expireAfterSeconds: 0 });
    await this.database.collection("jobs").createIndex({ userId: 1, dedupKey: 1 });
    await this.database.collection("jobs").createIndex({ userId: 1, status: 1 });
    await this.database.collection("applications").createIndex({ userId: 1, jobId: 1 });
    await this.database.collection("agent_events").createIndex({ runId: 1, at: 1 });
  }

  private users(): Collection<UserDoc> {
    return this.database.collection<UserDoc>(USERS);
  }
  private devices(): Collection<DeviceDoc> {
    return this.database.collection<DeviceDoc>(DEVICES);
  }
  private pairingCodes(): Collection<PairingCodeDoc> {
    return this.database.collection<PairingCodeDoc>(PAIRING_CODES);
  }
  private refreshTokens(): Collection<RefreshTokenDoc> {
    return this.database.collection<RefreshTokenDoc>(REFRESH_TOKENS);
  }
  private domain(collection: string): Collection<DomainDoc> {
    return this.database.collection<DomainDoc>(collection);
  }

  async createUser(id: string, email: string, passwordHash: string, displayName: string, now: Date): Promise<UserRecord> {
    const doc: UserDoc = { _id: id, email, passwordHash, displayName, createdAt: now.toISOString() };
    try {
      await this.users().insertOne(doc);
    } catch (e) {
      if (isDuplicateKeyError(e)) throw new Error("EMAIL_TAKEN");
      throw e;
    }
    return toUserRecord(doc);
  }

  async findUserByEmail(email: string) {
    const doc = await this.users().findOne({ email });
    if (!doc) return null;
    return { user: toUserRecord(doc), passwordHash: doc.passwordHash };
  }

  async findUserById(id: string): Promise<UserRecord | null> {
    const doc = await this.users().findOne({ _id: id });
    return doc ? toUserRecord(doc) : null;
  }

  async createDevice(
    id: string,
    userId: string,
    name: string,
    kind: DeviceKind,
    platform: string,
    tokenHash: string,
    now: Date,
  ): Promise<DeviceRecord> {
    const doc: DeviceDoc = { _id: id, userId, name, kind, platform, tokenHash, createdAt: now.toISOString(), lastSeenAt: null };
    await this.devices().insertOne(doc);
    return toDeviceRecord(doc);
  }

  async findDeviceByTokenHash(hash: string): Promise<DeviceRecord | null> {
    const doc = await this.devices().findOne({ tokenHash: hash });
    return doc ? toDeviceRecord(doc) : null;
  }

  async listDevices(userId: string): Promise<DeviceRecord[]> {
    const docs = await this.devices().find({ userId }).toArray();
    return docs.map(toDeviceRecord);
  }

  async deleteDevice(userId: string, deviceId: string): Promise<boolean> {
    const result = await this.devices().deleteOne({ _id: deviceId, userId });
    return result.deletedCount > 0;
  }

  async touchDeviceLastSeen(deviceId: string, now: Date): Promise<void> {
    await this.devices().updateOne({ _id: deviceId }, { $set: { lastSeenAt: now.toISOString() } });
  }

  async createPairingCode(code: string, userId: string, now: Date, expiresAt: Date): Promise<void> {
    try {
      await this.pairingCodes().insertOne({ _id: code, userId, expiresAt: expiresAt.toISOString(), redeemedAt: null });
    } catch (e) {
      if (isDuplicateKeyError(e)) throw new Error("CODE_COLLISION");
      throw e;
    }
    void now;
  }

  async redeemPairingCode(code: string, now: Date): Promise<string | null> {
    const doc = await this.pairingCodes().findOneAndUpdate(
      { _id: code, redeemedAt: null, expiresAt: { $gt: now.toISOString() } },
      { $set: { redeemedAt: now.toISOString() } },
    );
    return doc?.userId ?? null;
  }

  async insertRefreshToken(hash: string, userId: string, now: Date, expiresAt: Date): Promise<void> {
    await this.refreshTokens().insertOne({ _id: hash, userId, expiresAt: expiresAt.toISOString(), consumedAt: null });
    void now;
  }

  async consumeRefreshToken(hash: string, now: Date): Promise<string | null> {
    const doc = await this.refreshTokens().findOneAndUpdate(
      { _id: hash, consumedAt: null, expiresAt: { $gt: now.toISOString() } },
      { $set: { consumedAt: now.toISOString() } },
    );
    return doc?.userId ?? null;
  }

  async upsertDoc(collection: string, userId: string, id: string, value: JsonDoc): Promise<void> {
    const doc: DomainDoc = { ...value, id, _id: scopedId(userId, id), userId };
    await this.domain(collection).replaceOne({ _id: doc._id }, doc, { upsert: true });
  }

  async deleteDoc(collection: string, userId: string, id: string): Promise<void> {
    await this.domain(collection).deleteOne({ _id: scopedId(userId, id) });
  }

  async fetchAllDocs(collection: string, userId: string): Promise<JsonDoc[]> {
    const docs = await this.domain(collection).find({ userId }).toArray();
    return docs.map(stripMongoId);
  }

  async fetchDoc(collection: string, userId: string, id: string): Promise<JsonDoc | null> {
    const doc = await this.domain(collection).findOne({ _id: scopedId(userId, id) });
    return doc ? stripMongoId(doc) : null;
  }

  async fetchDocsByField(collection: string, userId: string, field: string, value: unknown): Promise<JsonDoc[]> {
    const docs = await this.domain(collection)
      .find({ userId, [field]: value } as Record<string, unknown>)
      .toArray();
    return docs.map(stripMongoId);
  }

  async close(): Promise<void> {
    await this.client.close();
  }
}

function stripMongoId(doc: DomainDoc): JsonDoc {
  const { _id, ...rest } = doc;
  void _id;
  return rest;
}

function toUserRecord(doc: UserDoc): UserRecord {
  return { id: doc._id, email: doc.email, displayName: doc.displayName, createdAt: doc.createdAt };
}

function toDeviceRecord(doc: DeviceDoc): DeviceRecord {
  return {
    id: doc._id,
    userId: doc.userId,
    name: doc.name,
    kind: doc.kind,
    platform: doc.platform,
    createdAt: doc.createdAt,
    lastSeenAt: doc.lastSeenAt,
  };
}

function isDuplicateKeyError(e: unknown): boolean {
  return typeof e === "object" && e !== null && "code" in e && (e as { code?: number }).code === 11000;
}
