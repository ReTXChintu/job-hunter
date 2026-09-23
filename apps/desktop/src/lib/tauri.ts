import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CommandArgs, CommandName, CommandResult, EventPayloads } from "@job-hunter/agent-protocol";
import type { UserFacingError } from "@job-hunter/types";

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export class BackendError extends Error {
  code: string;
  details: string | null;
  recoverable: boolean;
  constructor(err: UserFacingError) {
    super(err.message);
    this.name = "BackendError";
    this.code = err.code;
    this.details = err.details;
    this.recoverable = err.recoverable;
  }
}

function toBackendError(e: unknown): BackendError {
  if (e instanceof BackendError) return e;
  if (e && typeof e === "object" && "message" in e && "code" in e) {
    const err = e as UserFacingError;
    return new BackendError({ code: err.code ?? "OTHER", message: String(err.message), details: err.details ?? null, recoverable: err.recoverable ?? true });
  }
  return new BackendError({ code: "OTHER", message: typeof e === "string" ? e : e instanceof Error ? e.message : "Unknown error", details: null, recoverable: true });
}

/** Typed invoke: `await invoke("get_job", { id })`. */
export async function invoke<N extends CommandName>(name: N, args?: CommandArgs<N>): Promise<CommandResult<N>> {
  if (!isTauri()) {
    throw new BackendError({ code: "NOT_IN_TAURI", message: "Job Hunter must run inside the desktop application (pnpm tauri dev).", details: null, recoverable: false });
  }
  try {
    return (await tauriInvoke(name, args as Record<string, unknown>)) as CommandResult<N>;
  } catch (e) {
    throw toBackendError(e);
  }
}

export async function listen<E extends keyof EventPayloads>(event: E, handler: (payload: EventPayloads[E]) => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return tauriListen<EventPayloads[E]>(event, (e) => handler(e.payload));
}

export function errorMessage(e: unknown): string {
  return toBackendError(e).message;
}

export function errorDetails(e: unknown): string | null {
  return toBackendError(e).details;
}
