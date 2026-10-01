import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useKai } from "../state/kai-runtime";
interface EverythingInfo { status:string; installed:boolean; httpAvailable:boolean; activeProvider:string }
export function SettingsView({onClose}:{onClose:()=>void}) {
  const {settings,saveSettings}=useKai(); const [everything,setEverything]=useState<EverythingInfo|null>(null); const [name,setName]=useState(settings.displayName ?? ""); const [roots,setRoots]=useState((settings.searchRoots ?? []).join("; ")); const [memory,setMemory]=useState({recentCommands:[] as string[],recentSearches:[] as string[]}); const [notice,setNotice]=useState("");
  useEffect(()=>{void invoke<EverythingInfo>("everything_status").then(setEverything);void invoke<typeof memory>("get_memory").then(setMemory);},[]);
  const save=async(next:typeof settings)=>{try{await saveSettings(next);setNotice("Saved");}catch(e){setNotice(`Couldn’t save: ${String(e)}`)}}; const toggle=(key:"launchAtLogin"|"showIsland"|"reducedMotion"|"autoHideFullscreen")=>void save({...settings,[key]:!settings[key]});
  return <aside className="settings"><div className="panel-top"><strong>Kai settings</strong><button className="close" onClick={onClose} aria-label="Close settings">×</button></div>
    <h2>General</h2><label>What should Kai call you?<input type="text" value={name} maxLength={32} placeholder="Your name (optional)" onChange={e=>setName(e.target.value)} onBlur={()=>void save({...settings,displayName:name.trim()})}/></label><label><input type="checkbox" checked={settings.launchAtLogin} onChange={()=>toggle("launchAtLogin")}/>Start with Windows</label><label><input type="checkbox" checked={settings.reducedMotion} onChange={()=>toggle("reducedMotion")}/>Reduced motion</label>
    <h2>Appearance</h2><label><input type="checkbox" checked={settings.showIsland} onChange={()=>toggle("showIsland")}/>Show Kai</label><label><input type="checkbox" checked={settings.autoHideFullscreen} onChange={()=>toggle("autoHideFullscreen")}/>Auto-hide in fullscreen <em>planned</em></label>
    <h2>Search</h2><label>Provider<select value={settings.searchProvider} onChange={e=>void save({...settings,searchProvider:e.target.value as typeof settings.searchProvider})}><option value="auto">Automatic</option><option value="everything">Prefer Everything</option><option value="fallback">Local fallback</option></select></label><p>{everything?.status ?? "Checking Everything…"}</p><label>Extra fallback folders<input type="text" value={roots} placeholder="D:\Projects; D:\Archive" onChange={e=>setRoots(e.target.value)} onBlur={()=>void save({...settings,searchRoots:roots.split(";").map(v=>v.trim()).filter(Boolean)})}/></label>
    <h2>AI</h2><p>Online AI is not configured. Kai’s local tools remain available.</p>
    <h2>Privacy</h2><p>{memory.recentCommands.length} recent commands · {memory.recentSearches.length} recent searches</p><button className="settings-button" onClick={()=>void invoke("clear_memory").then(()=>{setMemory({recentCommands:[],recentSearches:[]});setNotice("Local history cleared")})}>Clear local history</button>
    <h2>About</h2><p>Kai Desktop v0.1.0 · {everything?.activeProvider ?? "Detecting search provider"}</p>{notice&&<p className="settings-notice">{notice}</p>}
  </aside>;
}
