export const companionStates = ["idle", "working", "thinking", "success", "error", "sleeping"] as const;

export type CompanionState = (typeof companionStates)[number];

export type CompanionEvent =
  | { type: "SET_STATE"; state: CompanionState; source?: string }
  | { type: "INTEGRATION_EVENT"; name: string; payload?: unknown };

export interface CompanionIntegration {
  readonly id: string;
  start?(dispatch: (event: CompanionEvent) => void): void | Promise<void>;
  stop?(): void | Promise<void>;
}
