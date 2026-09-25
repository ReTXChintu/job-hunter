# The backend (`@job-hunter/backend`)

This is a self-hosted Node service that owns the app's MongoDB Atlas data
directly, so the mobile app can read jobs and applications without the
desktop being online, and so the desktop's own persistence eventually goes
through one place instead of talking to MongoDB directly from Rust.

**Status: this is the one server in a deployment.** It serves the API, the
read-only web app (`apps/web`) at `/`, and the Android APK at
`/downloads/android`, reached as plain `http://<ip>:<port>` and run under
PM2 (see [`deploy.md`](deploy.md)). The desktop app syncs its local data
through it (Settings → Backend account) and routes the mobile app's live
connection through it too (Settings → Mobile app), since it speaks the
relay's protocol. Its address is baked into the desktop and mobile builds;
neither has a URL field. Signing in stays optional: without it the desktop
is local-only, exactly as before.

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
   scoped to the authenticated account, mirroring what the desktop's old
   direct MongoDB driver calls used to do (`upsert`/`delete`/`fetch_all`).
   This is the seam `crates/job-hunter-core/src/store/backend.rs` calls
   through instead of the MongoDB driver directly -- see "The Rust side"
   below.
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

## The Rust side (Phase 2, desktop half -- done)

`crates/job-hunter-core/src/store/backend.rs` replaced
`store/mongo.rs` entirely: same three operations
(`upsert`/`delete`/`fetch_all`), same call sites in `store/sync.rs`
(`SyncWorker::pull_all`/`flush`), just an HTTP client against
`/v1/data/...` with a stored device token instead of the MongoDB driver.
The local JSON-on-disk store (`store/local.rs`) and its sync queue are
unchanged -- the desktop still works fully offline; signing in only adds
somewhere for the existing queue to flush to.

`SyncWorker::configure()` no longer takes a raw MongoDB connection string.
It takes a backend URL, email and password, and does the same
register-or-login → register-device flow the mobile app's `AuthController`
does (`POST /v1/auth/register|login` → `POST /v1/devices/register`),
storing only the resulting device token (`secrets::BACKEND_DEVICE_TOKEN_KEY`)
-- never the password. This is Settings → Backend account (and Setup step
4) in the desktop UI, backed by the `test_backend`/`backend_register`/
`backend_login`/`backend_logout` Tauri commands.

Tested end-to-end in `crates/job-hunter-core/tests/backend_sync.rs`
against a real minimal HTTP server (not a mock) implementing the same
routes this backend exposes: sign-in, push a local write, pull a
remote-only write down, delete-and-sync, and a revoked-token failure path.

## Build-time server address

`crates/job-hunter-core/src/backend_url.rs` resolves the address the desktop
uses: `JOB_HUNTER_BACKEND_URL` at compile time (CI, from the `BACKEND_URL`
secret), else the same variable at run time (the root `.env` in
development), else `http://127.0.0.1:8788`. The Tauri commands pass it to
both `SyncWorker::configure` and `RemoteClient::register`/`login`, so there
is no way to point a release build anywhere else. A plain `http://` address
is accepted because the builder chose it. On start, `AppContext::init`
rewrites any previously saved address to the current one, so a signed-in
desktop follows the server to a new IP after an update.

The mobile app does the same with `--dart-define=BACKEND_URL=...`
(`apps/mobile/lib/config.dart`).

## The web app

`apps/web` is a small, read-only React app for checking on things from any
browser: an overview (counts, what needs you, whether the desktop is
online, the APK download), the application list and detail, and the job
list. It signs in with the same email and password (access token plus
refresh token, refreshed transparently) and reads only
`/v1/applications`, `/v1/data/jobs`, `/v1/data/job_analyses`, `/v1/devices`
and `/v1/downloads`. It never approves, rejects or applies; those stay in
the desktop and mobile apps. It uses hash routes (`#/applications/<id>`),
so the backend serves it as plain static files.

## Downloads and updates

`GET /v1/downloads` lists the newest APK and Windows installer in the
downloads folder, with the version (and Android build number) read from
their file names. `/downloads/android` and `/downloads/windows` serve them.
`/updates/windows/latest.json` is the manifest `tauri-plugin-updater` reads,
served only when the installer's `.sig` is present. The desktop
(`apps/desktop/src-tauri/src/updates.rs`) and mobile
(`apps/mobile/lib/state/update_controller.dart`) apps compare these against
their own versions. See [`deploy.md`](deploy.md) for the release flow.

## What's left

1. **Mobile reads without the desktop.** The mobile app now connects to
   this server, but still fetches data by asking a connected desktop over
   the WebSocket. Pointing its read side at `GET /v1/applications` (as the
   web app does) would let it show data while the desktop is off.
2. **Retire `job-hunter-relay`.** Nothing in a deployment uses the relay
   crate any more; this backend's `auth.ts`/`hub.ts` are direct ports of
   its `auth.rs`/`hub.rs`. It can be removed once nobody runs one.
3. **One sign-in on the desktop.** Backend account and Mobile app are
   still two separate sign-ins to the same account; they could share one.

## Deploying it

See [`deploy.md`](deploy.md).
