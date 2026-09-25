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
cp apps/backend/.env.example apps/backend/.env
```

Edit `apps/backend/.env`. The ones that matter:

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
| `DEPLOY_PATH` | Optional | Where the repo is cloned on the server, default `~/job-hunter`. |

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

## 3. Building the apps

`.github/workflows/build.yml` builds, in parallel:

- **Windows desktop**: NSIS `.exe` and `.msi` installers.
- **Android**: a release APK, after running the Flutter tests.

It runs when you push a version tag, and on demand from Actions → Build
apps → Run workflow.

```bash
git tag v0.1.0
git push origin v0.1.0
```

A tag build also creates a GitHub Release with the installers and the APK
attached. A manual run just leaves them as downloadable artifacts on the run.

## 4. Publishing the APK

The web app's Overview page shows a Download button once an APK is on the
server, and `/downloads/android` always serves the newest `.apk` in
`apps/backend/downloads/`.

- **Automatic**: set the `DEPLOY_*` secrets and every build copies its APK
  there, replacing the previous one.
- **Manual**: download the APK from the GitHub Release or the Actions run,
  then copy it into `apps/backend/downloads/` on the server, e.g.
  `scp job-hunter-*.apk user@<ip>:~/job-hunter/apps/backend/downloads/`.

No restart is needed either way.

## 5. Changing the server address

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
