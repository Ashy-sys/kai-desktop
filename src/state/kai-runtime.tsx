import { createContext, useContext, useEffect, useMemo, useState, type PropsWithChildren } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { CompanionState, IslandState, KaiEvent } from "../types/companion";

export type OpenAIModel = "gpt-5.6-luna" | "gpt-5.6-terra" | "gpt-5.6-sol";
export interface KaiSettings { displayName?: string; showCompanion: boolean; showIsland: boolean; islandAlwaysVisible: boolean; autoHideFullscreen: boolean; reducedMotion: boolean; launchAtLogin: boolean; searchProvider: "auto" | "everything" | "fallback"; searchRoots?: string[]; onlineAiEnabled: boolean; openaiModel: OpenAIModel; voiceResponses:boolean; computerControlEnabled:boolean; }
export interface SearchResult { name: string; path: string; extension: string | null; size: number | null; modified: string | null; isDirectory: boolean; }
interface Runtime { state: CompanionState; island: IslandState; connectivity: "online" | "offline" | "unknown"; settings: KaiSettings; dispatch(event: KaiEvent): Promise<void>; saveSettings(settings: KaiSettings): Promise<void>; }
const Context = createContext<Runtime | null>(null);

const islandFor = (event: KaiEvent): IslandState => { switch(event.kind) { case "stateChanged": return event.state === "thinking" ? "thinking" : event.state === "working" ? "working" : event.state === "success" ? "success" : event.state === "error" ? "error" : "idle"; case "searchStarted": return "searching"; case "searchCompleted": return "success"; case "taskStarted": return "working"; case "taskCompleted": return "success"; case "taskFailed": return "error"; case "connectivityChanged": return event.status === "offline" ? "offline" : "idle"; } };
export function KaiRuntimeProvider({ children }: PropsWithChildren) {
 const [state, setState] = useState<CompanionState>("idle"); const [island, setIsland] = useState<IslandState>("idle"); const [connectivity, setConnectivity] = useState<Runtime["connectivity"]>(navigator.onLine ? "online" : "offline");
 const [settings, setSettings] = useState<KaiSettings>({ showCompanion:false, showIsland:true, islandAlwaysVisible:true, autoHideFullscreen:false, reducedMotion:false, launchAtLogin:false, searchProvider:"auto", searchRoots:[], onlineAiEnabled:true, openaiModel:"gpt-5.6-luna",voiceResponses:false,computerControlEnabled:false });
 useEffect(() => { void invoke<KaiSettings>("get_settings").then(setSettings); let unlisten: (()=>void)|undefined; void listen<KaiEvent>("kai-event", ({ payload }) => apply(payload)).then(fn=>unlisten=fn); const online=()=>apply({kind:"connectivityChanged",status:"online"});const offline=()=>apply({kind:"connectivityChanged",status:"offline"});window.addEventListener("online",online);window.addEventListener("offline",offline);return () => {unlisten?.();window.removeEventListener("online",online);window.removeEventListener("offline",offline);}; }, []);
 const apply = (event: KaiEvent) => { setIsland(islandFor(event)); if (event.kind === "stateChanged") setState(event.state); if (event.kind === "connectivityChanged") setConnectivity(event.status); };
 const dispatch = async (event: KaiEvent) => { apply(event); await invoke("publish_kai_event", { event }); };
 const saveSettings = async (next: KaiSettings) => { setSettings(next); await invoke("save_settings", { settings: next }); document.documentElement.dataset.reduceMotion = String(next.reducedMotion); };
 return <Context.Provider value={useMemo(() => ({state,island,connectivity,settings,dispatch,saveSettings}),[state,island,connectivity,settings])}>{children}</Context.Provider>;
}
export const useKai = () => { const value = useContext(Context); if (!value) throw new Error("Kai runtime unavailable"); return value; };
