import { createContext, useContext, useMemo, useState, type PropsWithChildren } from "react";
import { info } from "@tauri-apps/plugin-log";
import type { CompanionState } from "../types/companion";

interface CompanionStore {
  state: CompanionState;
  setState: (next: CompanionState, source?: string) => void;
}

const CompanionContext = createContext<CompanionStore | null>(null);

export function CompanionProvider({ children }: PropsWithChildren) {
  const [state, updateState] = useState<CompanionState>("idle");
  const value = useMemo<CompanionStore>(() => ({
    state,
    setState: (next, source = "ui") => {
      updateState(next);
      void info(`Companion state: ${next} (${source})`);
    }
  }), [state]);
  return <CompanionContext.Provider value={value}>{children}</CompanionContext.Provider>;
}

export function useCompanion() {
  const context = useContext(CompanionContext);
  if (!context) throw new Error("useCompanion must be used inside CompanionProvider");
  return context;
}
