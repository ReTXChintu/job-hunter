import { useQueryClient } from "@tanstack/react-query";
import { createContext, useContext, useMemo, useState, type ReactNode } from "react";

import { ApiClient, loadSession, storeSession, type Session } from "./api";

interface SessionContextValue {
  api: ApiClient;
  session: Session | null;
}

const SessionContext = createContext<SessionContextValue | null>(null);

export function SessionProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const [session, setSession] = useState<Session | null>(() => loadSession());
  const api = useMemo(
    () =>
      new ApiClient(loadSession(), (next) => {
        storeSession(next);
        setSession(next);
        if (!next) queryClient.clear();
      }),
    [queryClient],
  );
  return <SessionContext.Provider value={{ api, session }}>{children}</SessionContext.Provider>;
}

export function useSession(): SessionContextValue {
  const ctx = useContext(SessionContext);
  if (!ctx) throw new Error("useSession must be used inside <SessionProvider>");
  return ctx;
}
