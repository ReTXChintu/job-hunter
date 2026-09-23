import type { AgentState, ApplicationStatus, JobListItem, JobStatus, ApplicationListItem, CandidateProfile } from "@job-hunter/types";

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

export function filterJobs(items: JobListItem[], filter: JobFilter, query = ""): JobListItem[] {
  const q = query.trim().toLowerCase();
  return items.filter((item) => {
    if (!matchesJobFilter(item, filter)) return false;
    if (!q) return true;
    const hay = `${item.job.title} ${item.job.company} ${item.job.location} ${item.job.skills.join(" ")}`.toLowerCase();
    return hay.includes(q);
  });
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
  COMPLETED: "Completed",
  FAILED: "Failed",
  MANUAL_ACTION_REQUIRED: "Manual action required",
  WAITING_FOR_USER: "Waiting for your input",
  PAUSED: "Paused",
  STOPPING: "Stopping",
};

export function isAgentRunning(state: AgentState): boolean {
  return ["INITIALIZING", "DISCOVERING", "EXTRACTING", "DEDUPLICATING", "ANALYZING", "PREPARING_APPLICATIONS", "APPLYING", "STOPPING", "PAUSED"].includes(state);
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

export function applicationActions(status: ApplicationStatus, agentBusy: boolean): ApplicationActions {
  return {
    canApprove: status === "READY_FOR_REVIEW" || status === "REJECTED",
    canReject: !["REJECTED", "WITHDRAWN", "APPLIED", "INTERVIEW", "OFFER"].includes(status),
    canApply: status === "APPROVED" && !agentBusy,
    canAnswer: status === "WAITING_FOR_USER" && !agentBusy,
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
