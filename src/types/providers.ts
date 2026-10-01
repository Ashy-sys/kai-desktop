export type KaiToolName = "search_files" | "get_file_metadata" | "open_file" | "reveal_file" | "open_known_app" | "get_system_info";

export interface AIRequest { prompt: string; allowedTools: readonly KaiToolName[]; }
export interface AIResponse { text: string; requestedTool?: { name: KaiToolName; arguments: unknown }; }
export interface AIProvider { readonly id: string; isConfigured(): Promise<boolean>; complete(request: AIRequest): Promise<AIResponse>; }

export interface CodexActivity { project?: string; operation: string; status: "working" | "success" | "error"; }
export interface CodexActivityProvider { readonly id: string; subscribe(listener: (activity: CodexActivity) => void): () => void; }
