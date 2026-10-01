import { useEffect, useRef, useState, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { KaiAnimatedCharacter, type KaiVisualState } from "../companion/KaiAnimatedCharacter";
import { useKai, type SearchResult } from "../state/kai-runtime";
import { SettingsView } from "./SettingsView";
import { motion } from "../animations/motion";
import "../animations/island.css";

type Mode = "micro" | "compact" | "stretch" | "expanded" | "content";
export function KaiIsland() {
  const { state, island, settings, dispatch } = useKai();
  const [mode,setMode] = useState<Mode>("compact");
  const [phase,setPhase] = useState("rest");
  const [query,setQuery] = useState("");
  const [results,setResults] = useState<SearchResult[]>([]);
  const [searched,setSearched] = useState(false);
  const [busy,setBusy] = useState(false);
  const [error,setError] = useState("");
  const [settingsOpen,setSettingsOpen] = useState(false);
  const [menu,setMenu] = useState(false);
  const lock = useRef(false);
  const input = useRef<HTMLInputElement>(null);
  const reduced = settings.reducedMotion || window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const expanded = mode === "expanded" || mode === "content";
  const visual: KaiVisualState = busy ? "searching" : ["thinking","working","success","error"].includes(island) ? island as KaiVisualState : state;
  const delay = (ms:number) => new Promise(resolve=>window.setTimeout(resolve,reduced ? 0 : ms));
  async function size(next:Mode) { await invoke("resize_island",{mode:next}); setMode(next); }
  async function toggle() {
    if(lock.current) return;
    lock.current=true;
    try {
      if(expanded) {
        setPhase("dismiss"); await delay(motion.dismiss);
        setSettingsOpen(false); setMenu(false);
        await size("stretch"); setPhase("close"); await delay(motion.morph);
        await size(state === "sleeping" ? "micro" : "compact");
      } else {
        await size("stretch"); setPhase("stretch"); await delay(motion.morph);
        await size(searched ? "content" : "expanded"); setPhase("unfold");
        await delay(motion.reveal); input.current?.focus();
      }
      setPhase("rest");
    } catch(e) { setError(String(e)); } finally { lock.current=false; }
  }
  useEffect(()=>{ if(!lock.current && !expanded) void size(state === "sleeping" ? "micro" : "compact").catch(console.error); },[state]);
  useEffect(()=>{ document.documentElement.dataset.reduceMotion=String(settings.reducedMotion); },[settings.reducedMotion]);
  async function search() {
    if(!query.trim() || busy) return;
    setBusy(true); setError("");
    try {
      await size("content"); setSearched(true);
      await dispatch({kind:"searchStarted",query});
      const found=await invoke<SearchResult[]>("search_files",{query}); setResults(found);
      await dispatch({kind:"searchCompleted",query,count:found.length});
    } catch(e) { setError(String(e)); await dispatch({kind:"taskFailed",message:"Search unavailable"}); }
    finally { setBusy(false); }
  }
  async function action(command:string,path:string) { try { await invoke(command,{path}); } catch(e) { setError(String(e)); } }
  const tokens = Object.fromEntries(Object.entries(motion).map(([key,value])=>[`--motion-${key}`,`${value}ms`])) as CSSProperties;
  return <main className={`kai-surface mode-${mode} phase-${phase} reaction-${visual}`} style={tokens}
    onKeyDown={e=>{if(e.key === "Escape" && expanded) void toggle();}}
    onPointerMove={e=>{const bounds=e.currentTarget.getBoundingClientRect();e.currentTarget.style.setProperty("--gaze-x",`${((e.clientX-bounds.left)/bounds.width-.5)*4}px`);}}
    onPointerLeave={e=>e.currentTarget.style.setProperty("--gaze-x","0px")}>
    <div className="surface-sheen" aria-hidden="true"/>
    <header className="kai-crown">
      <button className="creature-port" aria-label={expanded ? "Collapse Kai" : "Open Kai"} aria-expanded={expanded} onClick={()=>void toggle()}><KaiAnimatedCharacter state={visual}/></button>
      {expanded && <div className="greeting"><small>HERE WITH YOU</small><h1>{settings.displayName?.trim() ? `What's up, ${settings.displayName.trim()}?` : "What's on your mind?"}</h1></div>}
      <span className="presence-light" aria-hidden="true"/>
      {expanded && <button className="quiet-action more" aria-label="More options" onClick={()=>setMenu(!menu)}>···</button>}
    </header>
    {expanded && <section className="kai-unfold">
      {settingsOpen ? <SettingsView onClose={()=>setSettingsOpen(false)}/> : <>
        <form className="command-field" onSubmit={e=>{e.preventDefault();void search();}}><input ref={input} aria-label="Ask Kai or search this PC" value={query} onChange={e=>setQuery(e.target.value)} placeholder="Ask Kai or search this PC…" maxLength={120}/><button aria-label="Submit search" disabled={busy || !query.trim()}>↗</button></form>
        {!searched && <div className="soft-suggestions"><span>Start with a filename</span><button onClick={()=>{setQuery("invoice");input.current?.focus();}}>invoice</button><button onClick={()=>{setQuery("photo");input.current?.focus();}}>photo</button></div>}
        {searched && <div className="found-files" aria-live="polite"><p className="result-caption">{busy ? "Looking around your PC…" : results.length ? `${results.length} little discoveries` : "Nothing here yet. Try another name."}</p>{!busy && results.map(result=><article key={result.path}><span className="file-mark">{result.isDirectory ? "▱" : "◇"}</span><button onClick={()=>void action("open_file",result.path)}><strong>{result.name}</strong><small>{result.path}</small></button><button className="quiet-action" aria-label={`Reveal ${result.name}`} onClick={()=>void action("reveal_file",result.path)}>↗</button></article>)}</div>}
      </>}
      {error && <p className="surface-error" role="alert">{error}</p>}
      {menu && <nav className="surface-menu"><button onClick={()=>{setSettingsOpen(true);setMenu(false);void size("content");}}>Personalize</button><button onClick={()=>void getCurrentWindow().hide()}>Hide for now</button></nav>}
    </section>}
  </main>;
}
