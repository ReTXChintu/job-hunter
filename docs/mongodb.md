# MongoDB Atlas

Job Hunter is local-first: everything is written to JSON collections in the
app data directory first. Mirroring to MongoDB Atlas is optional and, as of
this doc, goes through one place: the self-hosted backend
(`apps/backend`, see [`backend.md`](backend.md)), not a direct connection
from the desktop app.

The desktop itself never holds an Atlas connection string. It signs in to
your backend account (Settings → Backend account, or Setup step 4) and
syncs through it; the backend is the only thing that talks to MongoDB
directly.

## Create a cluster

1. Create a free cluster at https://www.mongodb.com/atlas.
2. Create a database user with read/write access.
3. Add your backend server's IP (or `0.0.0.0/0` if it doesn't have a fixed
   one) under Network Access.
4. Copy the connection string: `mongodb+srv://<user>:<password>@<cluster>.mongodb.net/`.

This connection string goes into the **backend's** configuration
(`JOB_HUNTER_BACKEND_MONGODB_URI` in the repo's single root `.env`) when
you deploy it -- see [`deploy.md`](deploy.md) and
[`../.env.example`](../.env.example). It is
never entered into the desktop app.

## Connecting the desktop

Setup wizard step 4, or **Settings → Backend account**: enter your
backend's address, sign in (or create an account the first time), and the
desktop registers itself as a device on that account -- the same account
system the mobile app uses. You can skip this and stay local-only; nothing
else in the app requires it.

## Collections

```
users, candidate_profiles, experiences, projects, jobs, job_analyses, resumes,
cover_letters, applications, application_answers, agent_runs, agent_events, settings
```

Each document's id is the application-level `id` string, and every
document carries `userId` so the schema can grow to multiple accounts on
the same backend.

## Sync behaviour

- Every local write enqueues an upsert/delete in `_sync_queue.json`; the
  queue survives restarts.
- The desktop's sync worker flushes the queue when the backend responds,
  retries with exponential backoff (15 s → 5 min), and shows
  `Backend offline · N pending` in the status bar.
- On connect it pulls every collection from the backend and merges
  documents whose `updatedAt` is newer than the local copy.
- Nothing is ever deleted locally because of a sync failure.

## Removing the connection

**Settings → Backend account → Sign out** forgets this desktop's
credentials locally. Your account, its data, and any other signed-in
devices are unaffected. Local data on this machine is untouched.
