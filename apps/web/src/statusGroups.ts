import type { ApplicationListItem, ApplicationStatus } from "@job-hunter/types";

/** How the web app groups application statuses for counts and filters. */
export const STATUS_GROUPS = [
  { key: "all", label: "All", statuses: null },
  { key: "review", label: "Awaiting approval", statuses: ["READY_FOR_REVIEW"] },
  { key: "attention", label: "Needs attention", statuses: ["MANUAL_ACTION_REQUIRED", "WAITING_FOR_USER"] },
  { key: "progress", label: "In progress", statuses: ["APPROVED", "APPLYING"] },
  { key: "applied", label: "Applied", statuses: ["APPLIED"] },
  { key: "interviews", label: "Interviews & offers", statuses: ["INTERVIEW", "OFFER"] },
  { key: "closed", label: "Closed", statuses: ["REJECTED", "WITHDRAWN"] },
] as const satisfies readonly { key: string; label: string; statuses: readonly ApplicationStatus[] | null }[];

export type StatusGroupKey = (typeof STATUS_GROUPS)[number]["key"];

export function inGroup(item: ApplicationListItem, key: StatusGroupKey): boolean {
  const group = STATUS_GROUPS.find((g) => g.key === key);
  if (!group || !group.statuses) return true;
  return (group.statuses as readonly ApplicationStatus[]).includes(item.application.status);
}

export function countByGroup(items: ApplicationListItem[]): Record<StatusGroupKey, number> {
  const counts = Object.fromEntries(STATUS_GROUPS.map((g) => [g.key, 0])) as Record<StatusGroupKey, number>;
  for (const item of items) for (const g of STATUS_GROUPS) if (inGroup(item, g.key)) counts[g.key] += 1;
  return counts;
}

/** Newest activity first. */
export function byUpdatedDesc(a: ApplicationListItem, b: ApplicationListItem): number {
  return b.application.updatedAt.localeCompare(a.application.updatedAt);
}
