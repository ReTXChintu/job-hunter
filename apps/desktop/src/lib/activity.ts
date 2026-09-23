import { useSyncExternalStore } from "react";
import type { AgentEvent, LogEntry } from "@job-hunter/types";

/**
 * Tiny external store for high-frequency streams (agent activity, logs) so
 * they do not go through TanStack Query.
 */
class Feed<T extends { id: string }> {
  private items: T[] = [];
  private listeners = new Set<() => void>();
  constructor(private capacity: number) {}

  push(item: T) {
    if (this.items.some((i) => i.id === item.id)) return;
    this.items = [...this.items, item].slice(-this.capacity);
    this.listeners.forEach((l) => l());
  }

  replace(items: T[]) {
    this.items = items.slice(-this.capacity);
    this.listeners.forEach((l) => l());
  }

  clear() {
    this.items = [];
    this.listeners.forEach((l) => l());
  }

  get = () => this.items;

  subscribe = (l: () => void) => {
    this.listeners.add(l);
    return () => this.listeners.delete(l);
  };
}

export const activityFeed = new Feed<AgentEvent>(600);
export const logFeed = new Feed<LogEntry>(2000);

export function useActivity(): AgentEvent[] {
  return useSyncExternalStore(activityFeed.subscribe, activityFeed.get, activityFeed.get);
}

export function useLogFeed(): LogEntry[] {
  return useSyncExternalStore(logFeed.subscribe, logFeed.get, logFeed.get);
}
