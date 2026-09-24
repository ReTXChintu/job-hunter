export type DeviceKind = "desktop" | "mobile";

export interface UserRecord {
  id: string;
  email: string;
  displayName: string;
  createdAt: string;
}

export interface DeviceRecord {
  id: string;
  userId: string;
  name: string;
  kind: DeviceKind;
  platform: string;
  createdAt: string;
  lastSeenAt: string | null;
}

export interface DeviceView extends DeviceRecord {
  online: boolean;
}

/** The authenticated identity resolved from a bearer token. */
export interface AuthIdentity {
  userId: string;
}

/** Names of every domain collection the generic data API will accept. Kept
 * in sync BY HAND with `crates/job-hunter-core/src/domain/mod.rs::COLLECTIONS`
 * -- see docs/mobile-protocol.md's sync-both-sides convention. */
export const DOMAIN_COLLECTIONS = [
  "users",
  "candidate_profiles",
  "experiences",
  "projects",
  "jobs",
  "job_analyses",
  "resumes",
  "cover_letters",
  "applications",
  "application_answers",
  "agent_runs",
  "agent_events",
  "settings",
] as const;

export type DomainCollection = (typeof DOMAIN_COLLECTIONS)[number];

export function isDomainCollection(name: string): name is DomainCollection {
  return (DOMAIN_COLLECTIONS as readonly string[]).includes(name);
}
