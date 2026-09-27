import type { Notification } from "@job-hunter/types";

import { href } from "./route";

/**
 * Notifications come from the desktop (see the Rust `AppContext::notify`),
 * synced through the backend. The web app is read-only, so "read" here is
 * per browser: everything created after the last time the list was opened.
 */

const SEEN_KEY = "jobhunter.notifications.seenAt";
const ALERTED_KEY = "jobhunter.notifications.alertedAt";

function load(key: string): string {
  try {
    return localStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

function store(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Private mode etc.: the badge just resets on reload.
  }
}

export const seenAt = () => load(SEEN_KEY);
export const markAllSeen = (latest: string) => store(SEEN_KEY, latest);

/** Newest first. */
export function sortNewest(list: Notification[]): Notification[] {
  return [...list].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
}

export function unreadCount(list: Notification[], seen: string): number {
  return list.filter((n) => n.createdAt > seen).length;
}

/**
 * The notifications that arrived since this browser last alerted, oldest
 * first. The first call only records where we are, so opening the site
 * doesn't replay history as alerts.
 */
export function takeFresh(list: Notification[]): Notification[] {
  const latest = list.reduce((max, n) => (n.createdAt > max ? n.createdAt : max), "");
  const alerted = load(ALERTED_KEY);
  if (latest) store(ALERTED_KEY, latest > alerted ? latest : alerted);
  if (!alerted) return [];
  return list.filter((n) => n.createdAt > alerted).sort((a, b) => a.createdAt.localeCompare(b.createdAt));
}

/** Where a notification leads on the web, or null for desktop-only pages. */
export function notificationHref(n: Pick<Notification, "linkPage" | "linkId">): string | null {
  switch (n.linkPage) {
    case "application":
      return n.linkId ? href({ page: "application", id: n.linkId }) : href({ page: "applications" });
    case "applications":
      return href({ page: "applications" });
    case "jobs":
      return href({ page: "jobs" });
    default:
      return null;
  }
}
