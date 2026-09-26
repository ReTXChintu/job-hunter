/**
 * The real MongoDB store against a real (in-process) mongod, because the
 * in-memory stand-in used elsewhere can't catch Mongo-specific behaviour --
 * like the duplicate-key collision this file was written for.
 */
import { MongoClient } from "mongodb";
import { MongoMemoryServer } from "mongodb-memory-server";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { MongoDb, scopedId } from "../src/mongoDb.js";

let server: MongoMemoryServer;
let uri: string;
let dbCounter = 0;
const nextDbName = () => `jh_test_${Date.now()}_${dbCounter++}`;

beforeAll(async () => {
  server = await MongoMemoryServer.create();
  uri = server.getUri();
}, 300_000);

afterAll(async () => {
  await server?.stop();
});

const userDoc = (name: string) => ({ id: "local-user", displayName: name, createdAt: "2026-09-26T00:00:00Z", updatedAt: "2026-09-26T00:00:00Z" });

describe("MongoDb domain documents", () => {
  it("lets two accounts each store a document with the same id", async () => {
    const db = await MongoDb.connect(uri, nextDbName());
    // Every desktop install's user record has the id "local-user".
    await db.upsertDoc("users", "account-a", "local-user", userDoc("A"));
    await db.upsertDoc("users", "account-b", "local-user", userDoc("B"));

    expect(await db.fetchDoc("users", "account-a", "local-user")).toMatchObject({ id: "local-user", displayName: "A", userId: "account-a" });
    expect(await db.fetchDoc("users", "account-b", "local-user")).toMatchObject({ displayName: "B", userId: "account-b" });
    expect(await db.fetchAllDocs("users", "account-a")).toHaveLength(1);

    // Updates and deletes stay within their own account.
    await db.upsertDoc("users", "account-a", "local-user", userDoc("A2"));
    await db.deleteDoc("users", "account-b", "local-user");
    expect(await db.fetchDoc("users", "account-a", "local-user")).toMatchObject({ displayName: "A2" });
    expect(await db.fetchDoc("users", "account-b", "local-user")).toBeNull();
    await db.close();
  });

  it("returns documents without Mongo's internal key", async () => {
    const db = await MongoDb.connect(uri, nextDbName());
    await db.upsertDoc("jobs", "account-a", "job-1", { id: "job-1", title: "Engineer" });
    const [doc] = await db.fetchAllDocs("jobs", "account-a");
    expect(doc).toEqual({ id: "job-1", title: "Engineer", userId: "account-a" });
    await db.close();
  });

  it("re-keys documents written before ids were per-account, on connect", async () => {
    const dbName = nextDbName();
    const raw = new MongoClient(uri);
    await raw.connect();
    // The old layout: bare ids as the key.
    await raw.db(dbName).collection("users").insertOne({ _id: "local-user", id: "local-user", userId: "account-a", displayName: "A" } as never);
    await raw.db(dbName).collection("jobs").insertOne({ _id: "job-1", userId: "account-a", title: "No id field" } as never);
    await raw.close();

    const db = await MongoDb.connect(uri, dbName);
    expect(await db.fetchDoc("users", "account-a", "local-user")).toMatchObject({ displayName: "A" });
    expect(await db.fetchDoc("jobs", "account-a", "job-1")).toMatchObject({ id: "job-1", title: "No id field" });
    // ...and the account that used to collide can now write its own copy.
    await db.upsertDoc("users", "account-b", "local-user", userDoc("B"));
    expect(await db.fetchDoc("users", "account-b", "local-user")).toMatchObject({ displayName: "B" });
    // Running it again moves nothing.
    expect(await db.migrateLegacyDomainIds()).toBe(0);

    const check = new MongoClient(uri);
    await check.connect();
    const keys = (await check.db(dbName).collection("users").find({}).toArray()).map((d) => d._id).sort();
    expect(keys).toEqual([scopedId("account-a", "local-user"), scopedId("account-b", "local-user")]);
    await check.close();
    await db.close();
  });
});
