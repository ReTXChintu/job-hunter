import type { Submission } from "@job-hunter/types";
import { toaster } from "../components/ui/toaster";

/** Tell the user when a start request had to wait its turn. */
export function announce(s: Submission) {
  if (!s.queued) return;
  toaster.create({
    type: "info",
    title: `Queued (#${s.position})`,
    description: `${s.queued.title} starts when the agent is free.`,
    closable: true,
  });
}

/** Wrap a mutation's onSuccess so queued requests are announced first. */
export function announcing(then: () => void) {
  return (s: Submission) => {
    announce(s);
    then();
  };
}
