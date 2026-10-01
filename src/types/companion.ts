export const companionStates = ["idle", "working", "thinking", "success", "error", "sleeping"] as const;

export type CompanionState = (typeof companionStates)[number];

export type IslandState = "idle" | "thinking" | "working" | "success" | "error" | "offline" | "searching" | "music" | "download" | "codex";

export type CompanionEvent =
  | { type: "SET_STATE"; state: CompanionState; source?: string }
  | { type: "INTEGRATION_EVENT"; name: string; payload?: unknown };

export type KaiEvent =
  | { kind: "stateChanged"; state: CompanionState; source?: string }
  | { kind: "searchStarted"; query: string }
  | { kind: "searchCompleted"; query: string; count: number }
  | { kind: "taskStarted"; label: string }
  | { kind: "taskCompleted"; label: string }
  | { kind: "taskFailed"; message: string }
  | { kind: "connectivityChanged"; status: "online" | "offline" | "unknown" };

export interface CompanionIntegration {
  readonly id: string;
  start?(dispatch: (event: CompanionEvent) => void): void | Promise<void>;
  stop?(): void | Promise<void>;
}
