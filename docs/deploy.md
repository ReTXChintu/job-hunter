# Deploying the server and building the apps

One server runs everything that isn't the desktop app: the backend API
(accounts, data sync, mobile connectivity), the read-only web app, and the
Android APK download. It is reached as plain `http://<server-ip>:<port>`;
no domain or TLS certificate is needed.

```
http://<ip>:<port>/                    web app (sign in, view jobs and applications)
http://<ip>:<port>/downloads/android   the latest Android APK
http://<ip>:<port>/v1/...              API used by the desktop, mobile and web apps
```

The desktop installer and the APK are built by GitHub Actions with that
address baked in. Neither app has a server URL field.

## 1. Server setup

Needs Node.js 20.19+ (22 LTS recommended), pnpm, and git. The steps below
assume Linux; Windows works the same way.

```bash
git clone <your repo> ~/job-hunter
cd ~/job-hunter
pnpm install --frozen-lockfile
cp .env.example .env
```

The whole repo shares that one `.env` at its root. On the server, only its
"Server" section matters:

| Variable | Value |
| --- | --- |
| `JOB_HUNTER_BACKEND_PORT` | The port clients use, e.g. `8788`. |
| `JOB_HUNTER_BACKEND_MONGODB_URI` | Your MongoDB Atlas connection string. If `mongodb+srv://` fails with `querySrv ETIMEOUT`, use Atlas's non-SRV (`mongodb://host1,host2,host3/...`) string. |
| `JOB_HUNTER_BACKEND_JWT_SECRET` | **Required.** A long random value: `openssl rand -hex 32`. The server refuses to start without it. |

Allow the server's public IP in Atlas → Network Access, and open the port
in the server's firewall (for example `sudo ufw allow 8788/tcp`, plus your
cloud provider's security group).

Build and start:

```bash
pnpm server:build     # builds the web app and the backend
pnpm server:start     # starts it under PM2 (reloads it if already running)
```

Check it: `curl http://<ip>:<port>/healthz` should print `ok`, and the
web app should load in a browser.

### Day-to-day commands

All from the repo root:

| Command | Does |
| --- | --- |
| `pnpm server:start` | Start, or reload if already running |
| `pnpm server:stop` | Stop |
| `pnpm server:restart` | Restart, re-reading `.env` |
| `pnpm server:logs` | Tail the logs (Ctrl+C to exit) |
| `pnpm server:status` | Show whether it's running, memory, restarts |
| `pnpm server:update` | `git pull`, install, rebuild, and restart in one go |
| `pnpm server:delete` | Remove it from PM2 entirely |

To keep it running across reboots, run once:

```bash
npx pm2 save
npx pm2 startup      # prints a command to run with sudo; run it
```

The PM2 process runs a single instance on purpose. Device presence and
WebSocket routing live in memory, so it must not be clustered.

## 2. GitHub secrets

Repository → Settings → Secrets and variables → Actions → New repository
secret.

| Secret | Required | Value |
| --- | --- | --- |
| `BACKEND_URL` | Yes | `http://<server-ip>:<port>`, e.g. `http://203.0.113.10:8788`. Baked into both apps. |
| `ANDROID_KEYSTORE_BASE64` | Strongly recommended | Your release signing key, base64-encoded (see below). |
| `ANDROID_KEYSTORE_PASSWORD` | With the keystore | The keystore password. |
| `ANDROID_KEY_ALIAS` | With the keystore | The key alias, e.g. `job-hunter`. |
| `ANDROID_KEY_PASSWORD` | With the keystore | The key password. |
| `DEPLOY_HOST` | Optional | Server IP. With `DEPLOY_SSH_KEY`, CI copies each new APK to the server. |
| `DEPLOY_USER` | Optional | SSH user on the server. |
| `DEPLOY_SSH_KEY` | Optional | A private key whose public half is in that user's `~/.ssh/authorized_keys`. |
| `DEPLOY_PORT` | Optional | SSH port, default 22. |
| `DEPLOY_PATH` | Optional | Where the repo is cloned on the server, relative to the SSH user's home or absolute. Default `job-hunter`. |
| `TAURI_SIGNING_PRIVATE_KEY` | Recommended | Signs Windows builds so the desktop app can install updates itself (see below). |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | With that key | Its password (leave empty if it has none). |
| `TAURI_UPDATER_PUBKEY` | With that key | The matching public key, built into the app to verify updates. |

### The Android signing key

Android only installs an update over an existing app if both are signed with
the same key. Without `ANDROID_KEYSTORE_BASE64`, CI signs with a throwaway
debug key that differs on every run, so every new APK has to be installed
fresh after uninstalling the old one. Create a key once and keep it safe; if
it's lost, installed apps can never be updated again.

```bash
keytool -genkeypair -v -keystore job-hunter-release.jks \
  -keyalg RSA -keysize 2048 -validity 10000 -alias job-hunter
base64 -w0 job-hunter-release.jks      # paste the output into ANDROID_KEYSTORE_BASE64
```

On Windows PowerShell, use
`[Convert]::ToBase64String([IO.File]::ReadAllBytes("job-hunter-release.jks"))`
for the second step.

### The desktop update key

The desktop app installs updates itself only when they're signed with a
key whose public half is built into it. Create a key pair once:

```bash
pnpm --filter @job-hunter/desktop tauri signer generate -w job-hunter-updater.key
```

Put the contents of `job-hunter-updater.key` in `TAURI_SIGNING_PRIVATE_KEY`,
its password in `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, and the contents of
`job-hunter-updater.key.pub` in `TAURI_UPDATER_PUBKEY`. Keep the private key
safe: installed apps only accept updates signed with it.

Without it, the desktop app still notices new versions, but offers
**Download** (run the installer yourself) instead of **Install & restart**.

## 3. Releasing a version

Releases are cut from the repo root with [release-it](https://github.com/release-it/release-it):

```bash
pnpm release            # asks patch / minor / major, then does everything below
pnpm release:patch      # or pick non-interactively: release:minor, release:major
pnpm release:dry        # show what would happen without changing anything
pnpm versions:check     # confirm every app agrees on the version
```

A release, which must run on a clean `main`:

1. Bumps the version in the root `package.json` and, through
   `scripts/sync-versions.mjs`, in every app: `apps/*/package.json`,
   `apps/desktop/src-tauri/tauri.conf.json`, the Rust workspace version in
   `Cargo.toml` (and `Cargo.lock`), and `apps/mobile/pubspec.yaml`.
2. Commits that as `chore: release vX.Y.Z`, tags it `vX.Y.Z`, and pushes.
3. The tag starts the build below. CI refuses a tag that doesn't match the
   versions in the code, so always release with `pnpm release` rather
   than tagging by hand.

## 4. Building the apps

`.github/workflows/build.yml` builds, in parallel:

- **Windows desktop**: NSIS `.exe` and `.msi` installers.
- **Android**: a release APK, after running the Flutter tests.

It runs for each tag `pnpm release` pushes, and on demand from Actions →
Build apps → Run workflow. A tag build also creates a GitHub Release with
the installers and the APK attached; a manual run just leaves them as
artifacts on the run.

## 5. Publishing builds and in-app updates

The server hosts the latest builds, and both apps check it for updates: on
start, when brought back to the foreground (mobile), and every 6 hours.
They show a banner when a newer version is there. About (sidebar version
link on desktop, Settings → About on mobile) has a **Check for updates**
button.

| Path on the server | Serves |
| --- | --- |
| `apps/backend/downloads/job-hunter-<version>-<build>.apk` | `/downloads/android`, the mobile update |
| `apps/backend/downloads/Job Hunter_<version>_x64-setup.exe` | `/downloads/windows`, the desktop update |
| … `-setup.exe.sig` next to it | Enables **Install & restart** on the desktop |

- **Automatic**: set the `DEPLOY_*` secrets. Every build copies its APK,
  installer and signature there, replacing the previous ones.
- **Manual**: download them from the GitHub Release or the Actions run and
  copy them into `apps/backend/downloads/`, keeping the file names (the
  version is read from them).

No restart is needed either way.

On Android, **Download** fetches the APK in the browser; opening it
installs the update over the app, keeping its sign-in, provided the
signing key hasn't changed. On Windows, **Install & restart** downloads the
installer, checks its signature, installs it and reopens the app. Your data
is kept either way.

## 6. Changing the server address

The address is compiled into the apps, so moving to a new IP or port means
updating `BACKEND_URL` and building again. Installed apps that are already
signed in follow the new address after updating, with no need to sign in
again as long as it's the same server and database.

## Development

None of this is needed locally. Run `pnpm --filter @job-hunter/backend dev`
and `pnpm tauri dev`; the desktop app defaults to `http://127.0.0.1:8788`
(override with `JOB_HUNTER_BACKEND_URL` in the root `.env`). The web app
runs with `pnpm --filter @job-hunter/web dev` on http://localhost:5173 and
proxies API calls to that same local backend.
