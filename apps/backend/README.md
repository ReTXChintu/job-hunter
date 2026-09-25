# @job-hunter/backend

The self-hosted API server that owns the app's MongoDB Atlas data. It
replaces `job-hunter-relay`'s role for mobile connectivity (accounts,
devices, pairing codes, presence, a WebSocket for live requests) and adds
a generic, account-scoped data API so the mobile app can read jobs,
applications, and analyses directly -- without needing the desktop online
-- and so the desktop's own persistence layer can eventually go through
this backend too instead of talking to MongoDB directly.

See [`../../docs/backend.md`](../../docs/backend.md) for the full
architecture and rollout plan, and
[`../../docs/mobile-protocol.md`](../../docs/mobile-protocol.md) for the
wire format this preserves.

## Run it locally

```bash
# From the repo root: the whole repo shares one .env there.
cp .env.example .env   # then set JOB_HUNTER_BACKEND_MONGODB_URI and _JWT_SECRET
pnpm --filter @job-hunter/backend dev
```

Listens on `http://0.0.0.0:8788` by default.

## Tests

```bash
pnpm --filter @job-hunter/backend test
```

Route and logic tests run against `test/inMemoryDb.ts`, a real second
implementation of the same `Db` interface `MongoDb` uses -- not a mock --
so a test failure means the route logic is actually wrong, not that a
mock was configured incorrectly. This mirrors how
`crates/job-hunter-relay`'s tests exercise a real (if trivial) database
rather than mocking it out; MongoDB itself isn't practical to spin up
per-test the way SQLite was; there is no automated test against a real
MongoDB Atlas cluster, so verify manually against a real cluster before
relying on `MongoDb` in production.
