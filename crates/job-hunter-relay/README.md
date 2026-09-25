# job-hunter-relay

The small always-on service that lets the Job Hunter mobile app know
whether your desktop is online, and lets it send review actions (approve,
reject, answer a question, mark applied) to the desktop from anywhere.

It holds **no job data, no resumes, no Claude or Chrome access, no
MongoDB connection** — only accounts, paired devices, and live WebSocket
routing between exactly one desktop and any phones paired to the same
account. See [`../../docs/mobile-protocol.md`](../../docs/mobile-protocol.md)
for the wire format and [`../../docs/relay.md`](../../docs/relay.md) for
deployment instructions.

## Run it locally

```bash
# From the repo root: the whole repo shares one .env there.
cp .env.example .env   # then set JOB_HUNTER_RELAY_JWT_SECRET
cargo run -p job-hunter-relay
```

It listens on `http://0.0.0.0:8787` by default and keeps its SQLite
database next to the working directory (`relay.sqlite3`).

## Tests

```bash
cargo test -p job-hunter-relay
```

Includes real end-to-end tests (`tests/auth_flow.rs`) that start the actual
HTTP+WebSocket server on a random port and drive it with real sockets:
register → pair a phone with a one-time code → connect both → observe
presence → route a request/response → revoke a device and see its socket
close.
