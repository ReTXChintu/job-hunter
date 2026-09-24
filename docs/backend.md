# The backend (`@job-hunter/backend`)

This is a self-hosted Node service that owns the app's MongoDB Atlas data
directly, so the mobile app can read jobs and applications without the
desktop being online, and so the desktop's own persistence eventually goes
through one place instead of talking to MongoDB directly from Rust.

**Status: built and tested as a standalone service. Not yet wired into the
desktop app or the mobile app.** Today, both apps still use
[`job-hunter-relay`](../crates/job-hunter-relay) (see
[`relay.md`](relay.md)) exactly as before -- this document describes the
new backend and the integration work still ahead of it, not something
already live. See "Rollout plan" below.

## Why this exists

The mobile companion app originally reached the desktop only through a
relay that forwarded live requests to a connected desktop socket -- if the
desktop was closed, the phone could see nothing, not even its own
application history. The desktop asked for a backend that:

- Owns the database directly (MongoDB Atlas, the same cluster already in
  use), so mobile reads (which jobs are applied, which are in review) work
  regardless of whether the laptop is on.
- Is self-hosted and deployed by the person running Job Hunter, not a
  service anyone else operates.
- Never runs AI or drives Chrome -- that stays exactly where it is, in the
  desktop app, calling the user's own Claude CLI and Claude in Chrome. This
  backend only ever stores and serves data; it has no Claude, no browser,
  no Anthropic API key, same as every other part of this project.
- Keeps the desktop working with no internet at all: the desktop keeps a
  local cache and syncs to the backend when reachable, the same shape as
  today's local-store-plus-Atlas-sync design, just pointed at this backend
  instead of MongoDB directly (see "The Rust side" below).

## What's built (Phase 1)

`apps/backend` -- a Fastify + TypeScript service, source layout:

```
apps/backend/src/
  auth.ts        password hashing, tokens, JWT (ported from job-hunter-relay/src/auth.rs)
  rateLimit.ts    sliding-window limiter for auth endpoints
  hub.ts          in-memory presence/routing registry (ported from hub.rs)
  db.ts           the Db interface -- every persistence operation as one seam
  mongoDb.ts      the MongoDB Atlas implementation of Db
  models.ts       DeviceKind/DeviceRecord/UserRecord, the domain collection allow-list
  routes/
    auth.ts       register/login/refresh/logout
    devices.ts    register/list/delete a device
    pairing.ts    create/redeem a pairing code
    data.ts       generic collection CRUD + joined application read views
    ws.ts         the one WebSocket endpoint (presence, request routing, changed pushes)
```

It is **wire-compatible with the existing relay protocol** documented in
[`mobile-protocol.md`](mobile-protocol.md): the same REST endpoints, the
same request/response and error shapes, the same WebSocket envelope,
presence semantics, and `DESKTOP_OFFLINE` synthetic-reply behavior. The
already-built Flutter app's `ApiClient`/`RelayClient` need no changes to
talk to this backend instead of the relay for accounts, devices, pairing,
and presence.

It adds two things the relay never had:

1. **A generic data API** -- `PUT`/`DELETE`/`GET /v1/data/:collection[/:id]`,
   scoped to the authenticated account, mirroring
   `crates/job-hunter-core/src/store/mongo.rs`'s `upsert`/`delete`/`fetch_all`
   operations exactly. This is the seam the Rust sync layer will call
   through in Phase 2 instead of the MongoDB driver directly.
2. **Direct read views** -- `GET /v1/applications` and
   `GET /v1/applications/:id`, returning the same joined
   `ApplicationListItem`/`ApplicationDetail` shape `packages/types` and the
   mobile app's Dart models already expect, computed server-side from the
   generic collections. This is what lets the phone show "which jobs are
   applied, which are in review" without the desktop being reachable at
   all.

Every document keeps the `userId` field the domain types already define
(`packages/types`' `Job`, `Application`, etc. all have one); the backend
stamps it from the authenticated caller on every write, so one account can
never read or overwrite another's data even if a client tried to lie about
it.

Resume and cover-letter **file content** (the actual PDF/DOCX bytes) stays
out of scope for this backend and out of MongoDB entirely, matching how it
already works today -- only small metadata fields travel through the data
API and the joined read views.

### Testing

`apps/backend/test/inMemoryDb.ts` is a second, real implementation of the
`Db` interface (not a mock of individual calls), so route tests exercise
real logic without needing a MongoDB instance per test run -- see the
package's own [README](../apps/backend/README.md) for why. The WebSocket
tests spin up a real server and drive it with real `ws` clients, the same
standard the rest of this project holds itself to (compare
`crates/job-hunter-relay`'s and `crates/job-hunter-core`'s own real-socket
tests). All 36 tests pass; `pnpm --filter @job-hunter/backend build`
produces a runnable `dist/index.js`. There is no automated test against a
real MongoDB Atlas cluster -- verify manually before depending on it in
production.

## Rollout plan

Phase 1 (done, this document's subject) is additive and safe: it changes
nothing about how the desktop or mobile app currently behave.

**Phase 2 (not started)** is the actual integration, and it's substantial
enough to do as its own deliberate piece of work rather than bundled in
with standing the backend up:

1. **The Rust side.** `crates/job-hunter-core/src/store/mongo.rs` wraps
   exactly the operations this backend's data API exposes
   (`upsert`/`delete`/`fetch_all`). Swap it for an HTTP client hitting
   `/v1/data/...` with a stored device token, alongside the existing local
   JSON-on-disk store (`store/local.rs`) and its sync queue
   (`store/sync.rs`) -- the desktop keeps working offline exactly as it
   does today, syncing to this backend instead of to MongoDB directly, the
   moment it's reachable.
2. **Desktop login.** The desktop currently has no account of its own;
   `SyncWorker.configure()` takes a raw MongoDB connection string. That
   becomes "sign in to your Job Hunter account" (the same account system
   this backend already provides), issuing the desktop a device token the
   same way `/v1/devices/register` already does for the relay today.
3. **Mobile app.** Point `ApiClient`/`RelayClient` at this backend's URL
   instead of the relay's, and add direct calls to `GET /v1/applications`
   / `GET /v1/applications/:id` for the read side, so the app works even
   when the desktop is unreachable -- exactly the capability this backend
   was built to add. Approve/reject/apply/answer-questions can stay live
   WebSocket requests to a connected desktop (unchanged), or move to plain
   writes through the data API that the desktop picks up next time it's
   running -- a real design choice to make deliberately in Phase 2, not
   something Phase 1 commits to.
4. **Retire `job-hunter-relay`.** Once Phase 2 is live, the relay crate's
   accounts/devices/pairing/WebSocket-hub role is fully superseded by this
   backend (whose `auth.ts`/`hub.ts` are direct ports of the relay's own
   `auth.rs`/`hub.rs`) -- it can be removed rather than run alongside it.

Each of those is its own real chunk of work touching a different part of
the codebase (Rust core, Tauri UI, Flutter app respectively); Phase 1
deliberately stops short of them so the backend lands as a complete,
verified piece rather than a half-migrated one.

## Deploying it

See [`../apps/backend/README.md`](../apps/backend/README.md) for local
development and [`../apps/backend/.env.example`](../apps/backend/.env.example)
for configuration. Deployment shape mirrors [`relay.md`](relay.md) --
Docker image, HTTPS via a reverse proxy, point it at the same MongoDB
Atlas cluster the desktop app already uses.
