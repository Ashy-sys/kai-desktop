import type { PropsWithChildren } from "react";
import { KaiRuntimeProvider, useKai } from "./kai-runtime";

export const CompanionProvider = ({children}:PropsWithChildren) => <KaiRuntimeProvider>{children}</KaiRuntimeProvider>;

export function useCompanion() {
  const { state, dispatch } = useKai();
  return { state, setState: (next: import("../types/companion").CompanionState, source?: string) => void dispatch({kind:"stateChanged",state:next,source}) };
}
