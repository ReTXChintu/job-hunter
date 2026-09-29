# The mobile app

`apps/mobile` is a Flutter companion app: a phone-sized review surface for
applications your desktop agent has already prepared. It is intentionally
thin.

## What it can do

Five tabs:

- **Home** — whether the desktop is online, what the agent is doing
  (`agent_status` pushes / `get_agent_status`), counts of applications
  pending approval, needing input or a manual step, applied, interviews and
  offers (tap one to open that filter), what's waiting on you, and quick
  actions that run on the desktop: **Start job hunt**, **Stop**, **Find
  hiring posts** (a job hunt limited to LinkedIn Posts) and **Check Gmail for
  replies**. They're disabled, with the reason shown, while the desktop is
  offline or the agent is busy.
- **Applications** — every application, with filter chips and counts (All,
  Needs you, Pending approval, In progress, Applied, Interviews & offers,
  Rejected/withdrawn), search, and pull to refresh. The detail screen shows
  the status and its history, the posting, the match analysis, employers'
  replies found in Gmail (with an "In Spam" badge), the answers used, the
  failure reason and the tailored resume/cover letter, and offers what the
  status allows: **Approve & apply** / **Reject**, the **pending-questions
  form** (answers are saved for reuse), **Retry / resume** and **Mark as
  applied** for a manual step, and interview/offer/rejected/withdrawn
  tracking once applied.
- **Jobs** — everything the desktop found (All, Shortlisted, Not relevant,
  Has application, Email posts, plus search), each with its description,
  analysis and source link, and **Prepare application** / **Reject job**.
- **Alerts** — the desktop's notifications; tapping one opens the linked
  application, list, jobs or job sites.
- **More** — **Job sites** (each profile's state, Update / Resume / Start
  over, and the questions an update stopped on), **Additional details**
  (saved answers, add and edit), Settings and About.

Reads come live from the desktop when it's online. When it isn't, the app
reads what the desktop last synced to the server (`/v1/applications`,
`/v1/data/jobs`, ...) and says so ("Showing data from the server — desktop
offline"). Job-site profiles are desktop-only.

## What it deliberately cannot do

- **No AI runs on the phone.** Every action is a request the server
  forwards to the desktop, which calls the exact same `orchestrator`
  functions the desktop UI calls
  (`crates/job-hunter-core/src/remote/dispatch.rs`). The phone cannot bypass
  the desktop's approval gating (`domain/application.rs::can_transition_to`).
- **Actions need the desktop online.** The server never queues them: with
  the desktop offline the reply is `DESKTOP_OFFLINE` and the app says "Your
  desktop is offline — open Job Hunter on your computer".
- It cannot manage other paired devices or the account itself — that
  stays on the desktop's Settings → Mobile app tab.

## Notifications

- **App open:** the server pushes each notification over the WebSocket
  the moment the desktop syncs it; the app also catches up whenever it comes
  to the foreground or reconnects.
- **App closed, with Firebase:** the server sends a Firebase Cloud Messaging
  push, shown instantly on the `job_hunter_alerts` channel. The app registers
  its token with `PUT /v1/devices/me/push-token` on sign-in (and on token
  refresh) and clears it on sign-out.
- **App closed, without Firebase:** WorkManager checks
  `GET /v1/data/notifications` about every 15 minutes (Android may delay it).

All three paths share one `createdAt` watermark, so a notification is shown
once. Firebase is optional: the build applies the Google Services Gradle
plugin only when `android/app/google-services.json` exists (it's
git-ignored; CI writes it from the `GOOGLE_SERVICES_JSON` secret), and the
app starts normally without it. Setup is in
[deploy.md](deploy.md#phone-notifications-firebase).

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
              and the relay's wire format (Envelope, RelayResponse, ...);
              parsing never throws on a missing field (models/json.dart)
  logic/      Pure, tested rules: filter chips and counts, notification
              links, labels and error messages
  services/   ApiClient (REST), RelayClient (the WebSocket transport),
              SecureStore, NotificationService (local alerts + WorkManager),
              PushService (Firebase), LinkBus (notification taps)
  state/      ChangeNotifier controllers: Auth, Connection, Applications,
              Jobs, Agent, Profiles/Answers, Notifications, Nav
  theme/      Shared colors/status labels, kept in sync with the
              desktop's own theme and status vocabulary by hand
  widgets/    Small reusable presentation widgets, incl. the
              pending-questions form (question_form.dart)
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
