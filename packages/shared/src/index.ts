import type { AgentState, ApplicationStatus, JobListItem, JobStatus, ApplicationListItem, CandidateProfile, SkillGroups } from "@job-hunter/types";

export type JobFilter =
  | "ALL"
  | "NEW"
  | "RELEVANT"
  | "AWAITING_APPROVAL"
  | "APPROVED"
  | "APPLIED"
  | "REJECTED"
  | "MANUAL_ACTION"
  | "SAVED";

export const JOB_FILTERS: { key: JobFilter; label: string }[] = [
  { key: "ALL", label: "All" },
  { key: "NEW", label: "New" },
  { key: "RELEVANT", label: "Relevant" },
  { key: "AWAITING_APPROVAL", label: "Awaiting Approval" },
  { key: "APPROVED", label: "Approved" },
  { key: "APPLIED", label: "Applied" },
  { key: "REJECTED", label: "Rejected" },
  { key: "MANUAL_ACTION", label: "Manual Action" },
  { key: "SAVED", label: "Saved" },
];

export function matchesJobFilter(item: JobListItem, filter: JobFilter): boolean {
  const s = item.job.status;
  switch (filter) {
    case "ALL":
      return true;
    case "NEW":
      return s === "DISCOVERED";
    case "RELEVANT":
      return !!item.analysis?.relevant && !["REJECTED", "NOT_RELEVANT", "WITHDRAWN"].includes(s);
    case "AWAITING_APPROVAL":
      return s === "READY_FOR_REVIEW";
    case "APPROVED":
      return s === "APPROVED" || s === "APPLYING";
    case "APPLIED":
      return s === "APPLIED" || s === "INTERVIEW" || s === "OFFER";
    case "REJECTED":
      return s === "REJECTED" || s === "NOT_RELEVANT";
    case "MANUAL_ACTION":
      return s === "MANUAL_ACTION_REQUIRED" || s === "WAITING_FOR_USER";
    case "SAVED":
      return item.job.saved;
    default:
      return true;
  }
}

export function filterJobs(items: JobListItem[], filter: JobFilter, query = "", location = ""): JobListItem[] {
  const q = query.trim().toLowerCase();
  return items.filter((item) => {
    if (!matchesJobFilter(item, filter)) return false;
    if (!matchesLocation(item.job, location)) return false;
    if (!q) return true;
    const hay = `${item.job.title} ${item.job.company} ${item.job.location} ${item.job.source} ${item.job.skills.join(" ")}`.toLowerCase();
    return hay.includes(q);
  });
}

/** Applications in the given statuses (any, when undefined) that match the search and location. */
export function filterApplications(items: ApplicationListItem[], statuses: readonly ApplicationStatus[] | undefined, query = "", location = ""): ApplicationListItem[] {
  const q = query.trim().toLowerCase();
  return items.filter((item) => {
    if (statuses && !statuses.includes(item.application.status)) return false;
    if (!matchesLocation(item.job, location)) return false;
    if (!q) return true;
    return `${item.job.title} ${item.job.company} ${item.job.location} ${item.job.source}`.toLowerCase().includes(q);
  });
}

// ---- location filter: searches what's already in the database, never the web ----

export interface LocationOption {
  label: string;
  count: number;
}

const LOCATION_NOISE = new Set(["", "india", "in", "hybrid", "onsite", "on-site", "on site", "wfh", "work from home", "full time", "full-time", "part time", "part-time", "contract", "permanent", "anywhere"]);

/** Whether a job's location mentions remote work. */
function isRemote(job: { location: string; remote?: string | null }): boolean {
  return /remote|work from home|wfh/i.test(job.location) || /remote/i.test(job.remote ?? "");
}

/**
 * The places that appear in the jobs' locations, with how many jobs each
 * ("Mumbai, Maharashtra" and "Mumbai (Hybrid)" both count for Mumbai), most
 * common first. "Remote" is added for remote jobs.
 */
export function locationOptions(jobs: Iterable<{ location: string; remote?: string | null }>): LocationOption[] {
  const counts = new Map<string, LocationOption>();
  const bump = (label: string) => {
    const key = label.toLowerCase();
    const existing = counts.get(key);
    if (existing) existing.count += 1;
    else counts.set(key, { label, count: 1 });
  };
  for (const job of jobs) {
    const seen = new Set<string>();
    const parts = job.location
      .split(/[,/|•·()\[\]\-–—]|\band\b|\bor\b/i)
      .map((p) => p.replace(/\s+/g, " ").trim())
      .filter((p) => p.length >= 2 && !/^\d+$/.test(p) && !LOCATION_NOISE.has(p.toLowerCase()) && !/remote|work from home|wfh/i.test(p));
    for (const part of parts) {
      const key = part.toLowerCase();
      if (seen.has(key)) continue;
      seen.add(key);
      bump(part);
    }
    if (isRemote(job)) bump("Remote");
  }
  return [...counts.values()].sort((a, b) => b.count - a.count || a.label.localeCompare(b.label));
}

/** Whether a job is in the wanted place ("mumbai" matches "Mumbai, Maharashtra"; "remote" matches remote jobs). */
export function matchesLocation(job: { location: string; remote?: string | null }, wanted: string): boolean {
  const w = wanted.trim().toLowerCase();
  if (!w) return true;
  if (w === "remote") return isRemote(job);
  return job.location.toLowerCase().includes(w);
}

export function sortJobs(items: JobListItem[]): JobListItem[] {
  return [...items].sort((a, b) => {
    const sa = a.analysis?.matchScore ?? -1;
    const sb = b.analysis?.matchScore ?? -1;
    if (sb !== sa) return sb - sa;
    return b.job.discoveredAt.localeCompare(a.job.discoveredAt);
  });
}

export const STATUS_LABELS: Record<JobStatus | ApplicationStatus, string> = {
  DISCOVERED: "Discovered",
  ANALYZED: "Analyzed",
  NOT_RELEVANT: "Not relevant",
  SHORTLISTED: "Shortlisted",
  READY_FOR_REVIEW: "Awaiting approval",
  APPROVED: "Approved",
  APPLYING: "Applying",
  APPLIED: "Applied",
  MANUAL_ACTION_REQUIRED: "Manual action",
  WAITING_FOR_USER: "Needs your input",
  REJECTED: "Rejected",
  INTERVIEW: "Interview",
  OFFER: "Offer",
  WITHDRAWN: "Withdrawn",
};

export type StatusTone = "neutral" | "info" | "success" | "warning" | "danger" | "accent";

export function statusTone(status: JobStatus | ApplicationStatus): StatusTone {
  switch (status) {
    case "APPLIED":
    case "INTERVIEW":
    case "OFFER":
      return "success";
    case "READY_FOR_REVIEW":
    case "APPROVED":
    case "APPLYING":
      return "accent";
    case "MANUAL_ACTION_REQUIRED":
    case "WAITING_FOR_USER":
      return "warning";
    case "REJECTED":
    case "NOT_RELEVANT":
    case "WITHDRAWN":
      return "danger";
    case "SHORTLISTED":
    case "ANALYZED":
      return "info";
    default:
      return "neutral";
  }
}

export const AGENT_STATE_LABELS: Record<AgentState, string> = {
  IDLE: "Agent ready",
  INITIALIZING: "Starting",
  DISCOVERING: "Discovering jobs",
  EXTRACTING: "Reading job details",
  DEDUPLICATING: "Removing duplicates",
  ANALYZING: "Analyzing jobs",
  PREPARING_APPLICATIONS: "Preparing applications",
  WAITING_FOR_APPROVAL: "Waiting for your approval",
  APPLYING: "Applying",
  UPDATING_PROFILE: "Updating a job-site profile",
  CHECKING_INBOX: "Checking Gmail for replies",
  COMPLETED: "Completed",
  FAILED: "Failed",
  MANUAL_ACTION_REQUIRED: "Manual action required",
  WAITING_FOR_USER: "Waiting for your input",
  PAUSED: "Paused",
  STOPPING: "Stopping",
};

export function isAgentRunning(state: AgentState): boolean {
  return ["INITIALIZING", "DISCOVERING", "EXTRACTING", "DEDUPLICATING", "ANALYZING", "PREPARING_APPLICATIONS", "APPLYING", "UPDATING_PROFILE", "CHECKING_INBOX", "STOPPING", "PAUSED"].includes(state);
}

/** Which user actions make sense for an application in a given status. */
export interface ApplicationActions {
  canApprove: boolean;
  canReject: boolean;
  canApply: boolean;
  canAnswer: boolean;
  canMarkApplied: boolean;
  canApplyManually: boolean;
  canTrack: boolean;
}

/** A busy agent doesn't disable anything: the request waits in its queue. */
export function applicationActions(status: ApplicationStatus): ApplicationActions {
  return {
    canApprove: status === "READY_FOR_REVIEW" || status === "REJECTED",
    canReject: !["REJECTED", "WITHDRAWN", "APPLIED", "INTERVIEW", "OFFER"].includes(status),
    canApply: status === "APPROVED",
    canAnswer: status === "WAITING_FOR_USER",
    canMarkApplied: ["MANUAL_ACTION_REQUIRED", "WAITING_FOR_USER", "APPROVED"].includes(status),
    canApplyManually: ["MANUAL_ACTION_REQUIRED", "WAITING_FOR_USER", "APPROVED", "READY_FOR_REVIEW"].includes(status),
    canTrack: ["APPLIED", "INTERVIEW", "OFFER"].includes(status),
  };
}

export function pendingApplications(items: ApplicationListItem[]): ApplicationListItem[] {
  return items.filter((i) => ["READY_FOR_REVIEW", "WAITING_FOR_USER", "MANUAL_ACTION_REQUIRED"].includes(i.application.status));
}

// ---- candidate helpers -----------------------------------------------------------------

export function profileIssues(profile: CandidateProfile): string[] {
  const issues: string[] = [];
  if (!profile.personal.name.trim()) issues.push("Add your name");
  if (!profile.personal.email.trim()) issues.push("Add your email");
  else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(profile.personal.email.trim())) issues.push("Email address looks invalid");
  if (profile.preferences.targetRoles.length === 0) issues.push("Add at least one target role");
  const skillCount = Object.values(profile.skills).reduce((n, g) => n + g.length, 0);
  if (skillCount === 0) issues.push("Add your skills");
  return issues;
}

export function splitList(input: string): string[] {
  return input
    .split(/[,\n]/)
    .map((s) => s.trim())
    .filter((s, i, arr) => s.length > 0 && arr.indexOf(s) === i);
}

export function joinList(items: string[]): string {
  return items.join(", ");
}

// ---- formatting ---------------------------------------------------------------------------

export const INTERVIEW_MODE_LABELS: Record<string, string> = {
  PHONE: "Phone call",
  VIDEO: "Video call",
  ON_SITE: "On site",
  OTHER: "Other",
};

/** "Video call · HR screening", from whatever parts an interview has. */
export function interviewLine(interview: { mode: string; round: string; contact: string }): string {
  return [INTERVIEW_MODE_LABELS[interview.mode] ?? "", interview.round, interview.contact ? `with ${interview.contact}` : ""].filter(Boolean).join(" · ");
}

export function formatDateTime(iso: string | null | undefined): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString(undefined, { dateStyle: "medium" });
}

export function timeAgo(iso: string | null | undefined): string {
  if (!iso) return "";
  const d = new Date(iso).getTime();
  if (Number.isNaN(d)) return iso;
  const diff = Math.max(0, Date.now() - d);
  const m = Math.floor(diff / 60000);
  if (m < 1) return "just now";
  if (m < 60) return `${m} min ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h} h ago`;
  const days = Math.floor(h / 24);
  if (days < 30) return `${days} d ago`;
  return formatDate(iso);
}

export function greeting(date = new Date()): string {
  const h = date.getHours();
  if (h < 5) return "Good night";
  if (h < 12) return "Good morning";
  if (h < 17) return "Good afternoon";
  return "Good evening";
}

export function fileName(path: string | null | undefined): string {
  if (!path) return "";
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] ?? path;
}

export type SkillGroupKey = keyof SkillGroups;

export const SKILL_GROUPS: SkillGroupKey[] = ["frontend", "backend", "database", "devops", "cloud", "testing", "other"];

/** Checked in order; the first match wins, so specific rules come first. */
const SKILL_GROUP_RULES: [SkillGroupKey, RegExp][] = [
  ["testing", /\b(jest|mocha|chai|cypress|playwright|selenium|vitest|testing|tdd|qa)\b/i],
  ["database", /(sql|mongo|postgres|redis|dynamo|cassandra|elasticsearch|neo4j|firebase|supabase|prisma|graph database|database)/i],
  ["cloud", /\b(aws|azure|gcp|google cloud|lambda|beanstalk|cloudwatch|s3|ec2|heroku|vercel|netlify|cloud)\b/i],
  ["devops", /\b(docker|kubernetes|k8s|jenkins|ci\/cd|ci|cd|github actions|gitlab|terraform|ansible|nginx|pm2|linux|git)\b/i],
  ["frontend", /(react|next\.?js|angular|vue|svelte|redux|html|css|sass|tailwind|bootstrap|webpack|vite|frontend|front-end|\bui\b)/i],
  ["backend", /(node|express|nest\.?js|graphql|\brest\b|\bapis?\b|sockets?|django|flask|fastapi|spring|\bjava\b|python|golang|\bgo\b|php|laravel|ruby|rails|\.net|c#|kafka|rabbitmq|microservice|backend|back-end)/i],
];

/** A best guess at which skill group a skill belongs to (the user can change it). */
export function guessSkillGroup(skill: string): SkillGroupKey {
  for (const [group, pattern] of SKILL_GROUP_RULES) if (pattern.test(skill)) return group;
  return "other";
}

/**
 * Details job sites and application forms often require that a profile
 * doesn't hold. Answered once on the Candidate page, they're used for every
 * profile update and application.
 */
export const COMMON_QUESTIONS: string[] = [
  "What is your current annual CTC?",
  "What is your expected annual CTC?",
  "Can you join immediately? If not, your earliest joining date",
  "Are you currently serving a notice period? If yes, your last working day",
  "What is your date of birth?",
  "What is your gender?",
  "What is your marital status?",
  "What is your current city?",
  "What is your hometown?",
  "What is your permanent address with PIN code?",
  "Are you authorized to work in India?",
  "Do you require visa sponsorship?",
  "Do you have a valid passport?",
  "Are you open to rotational or night shifts?",
];

/** The same question in different wording, for matching saved answers. */
export function normalizeQuestion(q: string): string {
  return q
    .toLowerCase()
    .replace(/[^a-z0-9 ]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}
