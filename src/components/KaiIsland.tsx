import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useKai, type SearchResult } from "../state/kai-runtime";
export function KaiIsland() { const [open,setOpen]=useState(false), [query,setQuery]=useState(""), [results,setResults]=useState<SearchResult[]>([]); const {island,connectivity,dispatch}=useKai();
 async function search(){ if (!query.trim()) return; await dispatch({kind:"searchStarted",query}); const found=await invoke<SearchResult[]>("search_files",{query}); setResults(found); await dispatch({kind:"searchCompleted",query,count:found.length}); setOpen(true); }
 if (!open) return <button className={`island pill ${island}`} onClick={()=>setOpen(true)}><span className="dot"/> Kai <small>{connectivity === "offline" ? "offline" : ""}</small></button>;
 return <section className="island expanded"><header><button className="island-brand" onClick={()=>setOpen(false)}><span className="dot"/> Kai</button><span>{island === "searching" ? "Searching this PC…" : island === "offline" ? "Offline — local tools ready" : island === "success" ? "✓ Complete" : "Ready"}</span></header><form onSubmit={e=>{e.preventDefault();void search();}}><input value={query} onChange={e=>setQuery(e.target.value)} placeholder="Search this PC" maxLength={120}/><button>Search</button></form><div className="results">{results.map(result=><button key={result.path} onClick={()=>void invoke("open_file",{path:result.path})}><b>{result.name}</b><small>{result.path}</small></button>)}</div><footer><button onClick={()=>void getCurrentWindow().hide()}>Hide temporarily</button><button onClick={()=>void dispatch({kind:"stateChanged",state:"thinking",source:"island"})}>Test thinking</button></footer></section>;
}
