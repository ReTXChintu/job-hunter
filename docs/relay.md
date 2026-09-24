# Self-hosting the relay

`job-hunter-relay` is what lets the mobile app reach your desktop from
anywhere: it holds your account, your paired devices, and routes review
requests (approve, reject, answer a question) between the phone and the
desktop's already-running `orchestrator`. It never sees a job description,
a resume, a cover letter, or your Claude/MongoDB credentials — see
[`mobile-protocol.md`](mobile-protocol.md) for exactly what crosses it.

You host this yourself, on a machine you control. There is no shared
Job Hunter relay run by anyone else.

## What it needs

- A small, always-on host reachable from your phone: a cheap VPS, a
  Raspberry Pi on a reachable network, a container platform you already
  use. It's a single static binary plus a SQLite file — resource needs are
  trivial.
- HTTPS in front of it. Pairing and login send a password; both the
  desktop and mobile app refuse plaintext `http://` relay URLs unless you
  explicitly override that (and you shouldn't, outside of local testing).
  Put a reverse proxy (Caddy, nginx, Traefik) in front that terminates TLS
  and forwards to the relay's port, including WebSocket upgrades on
  `/v1/ws`.
- A place for its SQLite file to persist across restarts/redeploys.

## Configuration

All configuration is environment variables — see
[`crates/job-hunter-relay/.env.example`](../crates/job-hunter-relay/.env.example)
for the full list with defaults. The only one you must set yourself:

| Variable | Purpose |
|---|---|
| `JOB_HUNTER_RELAY_JWT_SECRET` | Signs access tokens. Generate with `openssl rand -hex 32`. A release build refuses to start without it; a debug build falls back to an insecure development value and logs a warning. |

Everything else has a sane default: bind address, database path, token
lifetimes, CORS origins, log verbosity.

## Running it

**Docker (recommended):**

```bash
docker build -t job-hunter-relay -f crates/job-hunter-relay/Dockerfile .
docker run -d \
  --name job-hunter-relay \
  -p 8787:8787 \
  -e JOB_HUNTER_RELAY_JWT_SECRET=$(openssl rand -hex 32) \
  -v job-hunter-relay-data:/data \
  job-hunter-relay
```

The image's `ENTRYPOINT` runs the binary directly; `/data` is a volume so
`relay.sqlite3` survives container recreation. Put your reverse proxy in
front of port 8787.

**Bare binary:**

```bash
cargo build --release -p job-hunter-relay
JOB_HUNTER_RELAY_JWT_SECRET=$(openssl rand -hex 32) \
JOB_HUNTER_RELAY_DB=/var/lib/job-hunter-relay/relay.sqlite3 \
  ./target/release/job-hunter-relay
```

Run it under whatever process supervisor you already use (systemd,
supervisord, a container orchestrator) so it restarts on crash/reboot.

**Local development** (talking to a desktop app and mobile app both
running on your own machine):

```bash
cd crates/job-hunter-relay
cp .env.example .env   # then edit JOB_HUNTER_RELAY_JWT_SECRET
cargo run -p job-hunter-relay
```

It listens on `http://0.0.0.0:8787` by default. The desktop and mobile
apps both let you type `http://` for local testing, with a visible warning
that it's insecure for anything beyond your own machine.

## Reverse proxy example (Caddy)

```
relay.example.com {
    reverse_proxy localhost:8787
}
```

Caddy handles the TLS certificate and WebSocket upgrade transparently;
nginx/Traefik need the usual explicit `Upgrade`/`Connection` header
passthrough for the `/v1/ws` path.

## Operating it

- **Backups**: back up the SQLite file (accounts, device tokens, pairing
  codes — no job data). Losing it means every paired phone has to be
  re-paired and everyone re-registers their account; nothing about your
  job search itself is lost, since none of it lives here.
- **Updating**: pull a new image/binary and restart; the schema is
  migrated automatically on startup.
- **Revoking a device**: don't do it here. Use the desktop app's Settings
  → Mobile app → Paired devices list, which calls the same relay endpoint
  under the hood.
- **Multiple desktops/phones**: one relay, one account. Every device you
  register or pair (desktop or phone) is a peer on that same account; the
  relay ensures a phone only ever reaches devices on its own account.

## Verifying it's up

```bash
curl -i https://relay.example.com/healthz
```

A `200` confirms the HTTP side is reachable through your proxy; the real
test is pairing a phone from Settings → Mobile app on the desktop and
watching the connection status turn green.
