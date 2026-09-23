import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { listen } from "./tauri";
import { activityFeed, logFeed } from "./activity";
import { COLLECTION_KEYS, keys } from "./queries";

/**
 * Mounted once at the root: subscribes to backend events and keeps query
 * caches and the activity/log feeds fresh.
 */
export function BackendEvents() {
  const qc = useQueryClient();
  useEffect(() => {
    const unlisteners: Array<Promise<() => void>> = [];
    unlisteners.push(listen("agent:event", (ev) => activityFeed.push(ev)));
    unlisteners.push(
      listen("agent:status", (status) => {
        qc.setQueryData(keys.agentStatus, status);
        if (["COMPLETED", "FAILED", "WAITING_FOR_APPROVAL", "MANUAL_ACTION_REQUIRED", "WAITING_FOR_USER"].includes(status.state)) {
          void qc.invalidateQueries({ queryKey: keys.dashboard });
          void qc.invalidateQueries({ queryKey: keys.runs });
          void qc.invalidateQueries({ queryKey: keys.jobs });
          void qc.invalidateQueries({ queryKey: keys.applications });
        }
      }),
    );
    unlisteners.push(
      listen("sync:status", (status) => {
        qc.setQueryData(keys.sync, status);
        void qc.invalidateQueries({ queryKey: keys.setup });
      }),
    );
    unlisteners.push(listen("log:entry", (entry) => logFeed.push(entry)));
    unlisteners.push(
      listen("data:changed", (collections) => {
        for (const c of collections) {
          for (const key of COLLECTION_KEYS[c] ?? []) void qc.invalidateQueries({ queryKey: key });
        }
      }),
    );
    return () => {
      for (const p of unlisteners) void p.then((u) => u());
    };
  }, [qc]);
  return null;
}
