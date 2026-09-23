# MongoDB Atlas

Job Hunter is local-first: everything is written to JSON collections in the app data directory first, then mirrored to MongoDB Atlas.

## Create a cluster

1. Create a free cluster at https://www.mongodb.com/atlas.
2. Create a database user with read/write access.
3. Add your machine's IP (or `0.0.0.0/0` for a personal laptop that changes networks) under Network Access.
4. Copy the connection string: `mongodb+srv://<user>:<password>@<cluster>.mongodb.net/`.

## Configure in Job Hunter

Setup wizard step 4 or **Settings → MongoDB Atlas**: paste the connection string, set the database name (default `job_hunter`), press **Test connection** then **Save & connect**.

- The URI is stored with the `keyring` crate in Windows Credential Manager / macOS Keychain / Linux Secret Service, never in a file. The React UI never receives it.
- `MONGODB_URI` in a `.env` file is honoured for development only.
- On save, all existing local documents are queued for upload; indexes on `jobs`, `applications` and `agent_events` are created.

## Collections

```
users, candidate_profiles, experiences, projects, jobs, job_analyses, resumes,
cover_letters, applications, application_answers, agent_runs, agent_events, settings
```

Each document's `_id` is the application-level `id` string, and every document carries `userId` so the schema can grow to multiple users.

## Sync behaviour

- Every local write enqueues an upsert/delete in `_sync_queue.json`; the queue survives restarts.
- `SyncWorker` flushes the queue when Atlas responds, retries with exponential backoff (15 s → 5 min) and shows `Atlas offline · N pending` in the status bar.
- On connect it pulls every collection and merges documents whose `updatedAt` is newer than the local copy.
- Nothing is ever deleted locally because of a sync failure.

## Removing the connection

**Settings → MongoDB Atlas → Remove connection** deletes the credential from the OS store. Local data is untouched.
