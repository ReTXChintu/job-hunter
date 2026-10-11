import { useMutation, useQueryClient, type QueryKey } from "@tanstack/react-query";

/**
 * One action sent to the desktop. `key` names which button started it (for
 * its spinner), `success` is what to say once the desktop has replied.
 */
export interface DesktopAction {
  key: string;
  run: () => Promise<unknown>;
  success: string;
}

/**
 * Runs actions on the desktop one at a time. Nothing is shown as done until
 * the desktop replies; afterwards the given queries are refetched, success
 * or not, since the desktop may have changed something either way.
 */
/** A request the desktop queued says so, instead of the "started" message. */
export function outcomeMessage(data: unknown, started: string): string {
  if (data && typeof data === "object" && (data as { queued?: boolean }).queued === true) {
    const { position, title } = data as { position?: number; title?: string };
    return `Queued on your desktop${position ? ` (#${position})` : ""}${title ? `: ${title}` : ""}. It starts when the agent is free.`;
  }
  return started;
}

export function useDesktopAction(refresh: QueryKey[]) {
  const queryClient = useQueryClient();
  const mutation = useMutation({
    mutationFn: (action: DesktopAction) => action.run(),
    onSettled: () => Promise.all(refresh.map((queryKey) => queryClient.invalidateQueries({ queryKey }))),
  });
  return {
    mutation,
    busy: mutation.isPending,
    /** Whether the running action is the one started by `key`. */
    running: (key: string) => mutation.isPending && mutation.variables?.key === key,
    success: mutation.isSuccess ? outcomeMessage(mutation.data, mutation.variables.success) : null,
    error: mutation.error,
    run: (action: DesktopAction, options?: { onSuccess?: () => void }) => mutation.mutate(action, options),
    reset: () => mutation.reset(),
  };
}
