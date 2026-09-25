# The mobile app

`apps/mobile` is a Flutter companion app: a phone-sized review surface for
applications your desktop agent has already prepared. It is intentionally
thin.

## What it can do

- See every application the agent has produced or is working on, with its
  status, match score, and the desktop's own analysis of why it matched.
- Open one application to read the job posting, the analysis, resume/cover
  letter info, and any potential issues the agent flagged.
- Approve or reject an application awaiting review.
- Answer a pending question the agent needs before it can continue.
- Confirm a manual step is done, or trigger "apply now" on an approved
  application.
- Show whether the desktop is currently online, reported by the relay, not
  guessed from the phone's own connectivity.
- Work offline in a read-only sense: the last-loaded list is cached on the
  phone and shown (labeled "Showing cached data") until a live connection
  refreshes it.

## What it deliberately cannot do

- **No AI runs on the phone.** Every action it sends is a request that the
  relay forwards to the desktop, which calls the exact same
  `orchestrator` functions the desktop UI itself calls
  (`crates/job-hunter-core/src/remote/dispatch.rs`). The phone has no
  Claude access, no browser automation, and cannot bypass the desktop's
  own approval gating (`domain/application.rs::can_transition_to`).
- **No job discovery, scraping, or job data storage.** It only ever reads
  what the desktop already produced, through the relay.
- It cannot manage other paired devices or the account itself — that
  stays on the desktop's Settings → Mobile app tab, which is the source of
  truth for who's paired.
- **If the desktop is offline, nothing works.** The relay only routes
  live requests to a connected desktop; it never queues actions or runs
  anything on its own.

## Architecture in one paragraph

The phone talks REST + WebSocket to a `job-hunter-relay` instance you
self-host (see [`relay.md`](relay.md)). REST handles account
register/login and device pairing; a single WebSocket connection per
device carries requests, their responses, and two kinds of pushes —
`presence` (is the desktop online) and `changed` (which collections moved,
so the phone knows to refresh). The full wire format is in
[`mobile-protocol.md`](mobile-protocol.md). The relay itself holds only
accounts, device records, and pairing codes — never job data.

## Signing in

Two ways to connect a phone to your account, both requiring your relay's
URL:

1. **Pair with a code (default)** — on the desktop, go to Settings →
   Mobile app → Generate pairing code, then either scan the QR code on the
   phone's "Pair with code" tab or type the code shown. No password is
   ever typed on the phone this way; the code is single-use and expires in
   five minutes.
2. **Email + password** — sign in (or create an account) directly on the
   phone. Useful for the very first device on a new account, or if pairing
   isn't convenient.

Either path ends the same way: the phone gets a long-lived device token,
stored in the platform's secure storage (`flutter_secure_storage`), and
never touches the account password again. Signing out on the phone
forgets that token locally; the device stays listed on the desktop's
Paired Devices panel until explicitly revoked from there.

## Building it

```bash
cd apps/mobile
flutter pub get
flutter analyze
flutter test
```

Platform scaffolding for Android and iOS is checked in
(`apps/mobile/android`, `apps/mobile/ios`); build for a connected device
or emulator the normal Flutter way:

```bash
flutter run                 # whatever device/emulator is connected
flutter build apk --release --dart-define=BACKEND_URL=http://<server-ip>:<port>
flutter build ios           # iOS (on macOS, with signing configured)
```

The server address is compiled in with `--dart-define=BACKEND_URL=...`
(see `lib/config.dart`). With it, the app has no server URL field; without
it (a plain `flutter run`), the sign-in and pairing screens ask for one,
which is only meant for development. Release APKs are normally built by CI
from the `BACKEND_URL` secret: see [deploy.md](deploy.md).

A Windows desktop build also works as a quick sanity check without a
phone or emulator (`flutter create --platforms=windows .` once, then
`flutter build windows`) — it's the same Dart/Flutter app, just not how
it's meant to be used day to day.

## Code layout

```
lib/
  models/     Hand-written JSON models mirroring the Rust domain types
              and the relay's wire format (Envelope, RelayResponse, ...)
  services/   ApiClient (REST), RelayClient (the WebSocket transport),
              SecureStore (device token + settings persistence)
  state/      ChangeNotifier controllers: AuthController,
              ConnectionController, ApplicationsController
  theme/      Shared colors/status labels, kept in sync with the
              desktop's own theme and status vocabulary by hand
  widgets/    Small reusable presentation widgets
  screens/    One file per screen
  app.dart    Routing (go_router) with an auth-gated redirect
  main.dart   Provider wiring and entry point
```

Models are hand-written rather than code-generated, matching the
convention already used in `packages/types` on the desktop side — when the
Rust domain types change, update both `packages/types` and
`apps/mobile/lib/models` together.

## Known limitations

This app has been verified with `flutter analyze` (no issues), the Dart
test suite (`flutter test`, pure-Dart unit and widget tests, no emulator
required), and a real `flutter build windows` that was launched and
screenshotted to confirm the login/pairing screen renders correctly. It
has **not** been run on a real Android or iOS device or emulator in this
environment, and the end-to-end pairing/approve/reject flow against a live
relay and desktop has not been exercised from the phone side — only the
desktop-to-relay half of that path was verified with real sockets (see
`crates/job-hunter-core/tests/remote_relay.rs`). Test on a real device
before relying on it.
