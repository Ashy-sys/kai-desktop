import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useKai } from "../state/kai-runtime";

export function SettingsView({onClose}:{onClose:()=>void}) {
  const {settings,saveSettings}=useKai();
  const [everything,setEverything]=useState("checking…");
  const [name,setName]=useState(settings.displayName ?? "");
  const [error,setError]=useState("");
  useEffect(()=>{void invoke<{status:string}>("everything_status").then(v=>setEverything(v.status)).catch(()=>setEverything("unavailable"));},[]);
  const save=(next:typeof settings)=>void saveSettings(next).catch(e=>setError(String(e)));
  return <aside className="settings"><div className="panel-top"><strong>Make yourself at home</strong><button className="close" onClick={onClose} aria-label="Close settings">×</button></div>
    <label>What should Kai call you?<input type="text" value={name} maxLength={32} placeholder="Your name (optional)" onChange={e=>setName(e.target.value)} onBlur={()=>save({...settings,displayName:name.trim()})}/></label>
    {([['showIsland','Show Kai Island'],['reducedMotion','Reduced motion']] as const).map(([key,label])=><label key={key}><input type="checkbox" checked={settings[key]} onChange={()=>save({...settings,[key]:!settings[key]})}/>{label}</label>)}
    <label>Search provider<select value={settings.searchProvider} onChange={e=>save({...settings,searchProvider:e.target.value as typeof settings.searchProvider})}><option value="auto">Auto</option><option value="everything">Everything when available</option><option value="fallback">Local fallback</option></select></label><p>Everything: {everything}</p>{error && <p role="alert">{error}</p>}
  </aside>;
}
