import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import kaiAsset from "../assets/kai-placeholder.png";
import { useKai, type SearchResult } from "../state/kai-runtime";
import { SettingsView } from "./SettingsView";

export function KaiIsland() {
  const [open, setOpen] = useState(false); const [query, setQuery] = useState(""); const [results, setResults] = useState<SearchResult[]>([]); const [settingsOpen,setSettingsOpen]=useState(false);
  const { state, island, connectivity, dispatch } = useKai();
  const resize = async (expanded:boolean) => { await invoke("resize_island", { expanded }); setOpen(expanded); };
  useEffect(()=> { void resize(false); }, []);
  async function search() { if (!query.trim()) return; await dispatch({ kind:"searchStarted", query }); const found = await invoke<SearchResult[]>("search_files", { query }); setResults(found); await dispatch({kind:"searchCompleted",query,count:found.length}); }
  const label = island === "searching" ? "Searching this PC…" : island === "thinking" ? "Kai is thinking…" : island === "working" ? "Working…" : island === "success" ? "Complete" : island === "error" ? "Something went wrong" : state === "sleeping" ? "Sleeping" : connectivity === "offline" ? "Offline" : "Kai";
  return <main className={open ? "kai-island open" : "kai-island compact"}>
    <button className={`island-trigger state-${state}`} onClick={()=>void resize(!open)} aria-label="Open Kai Island"><img src={kaiAsset} alt="Kai" draggable={false}/><span>{label}</span>{connectivity === "offline" && <i>•</i>}<b>{open ? "⌃" : "⌄"}</b></button>
    {open && <section className="island-content"><form onSubmit={event=>{event.preventDefault();void search();}}><input value={query} onChange={event=>setQuery(event.target.value)} placeholder="Ask or search this PC" maxLength={120} autoFocus/><button>Search</button></form>{results.length > 0 && <div className="results">{results.map(result=><article key={result.path}><button onClick={()=>void invoke("open_file",{path:result.path})}><b>{result.name}</b><small>{result.path}</small></button><button className="reveal" title="Reveal in Explorer" onClick={()=>void invoke("reveal_file",{path:result.path})}>⌁</button></article>)}</div>}{results.length === 0 && <p className="island-hint">Local search, commands, and status—always on your PC.</p>}<footer><button onClick={()=>setSettingsOpen(true)}>Settings</button><button onClick={()=>void getCurrentWindow().hide()}>Hide</button>{import.meta.env.DEV && <button onClick={()=>void dispatch({kind:"stateChanged",state:"thinking",source:"debug"})}>Debug state</button>}</footer>{settingsOpen && <SettingsView onClose={()=>setSettingsOpen(false)}/>}</section>}
  </main>;
}
