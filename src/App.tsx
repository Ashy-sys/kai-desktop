import { useEffect, useState } from "react";
import { KaiCharacter } from "./companion/KaiCharacter";
import { InteractionPanel } from "./components/InteractionPanel";
import { restoreWindowPosition } from "./services/window-service";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { KaiIsland } from "./components/KaiIsland";
import { SettingsView } from "./components/SettingsView";
import { useKai } from "./state/kai-runtime";

export default function App() {
  const [panelOpen, setPanelOpen] = useState(false);
  const [isIsland, setIsIsland] = useState(false); const [settingsOpen,setSettingsOpen]=useState(false); const {settings}=useKai();
  useEffect(() => { setIsIsland(getCurrentWindow().label === "island"); void restoreWindowPosition(); }, []);
  if (isIsland) return settings.showIsland ? <KaiIsland /> : null;
  if (!settings.showCompanion) return null;
  return <main className={panelOpen ? "app panel-open" : "app"} onContextMenu={(event) => { event.preventDefault(); setPanelOpen(true); }}>
    <KaiCharacter onActivate={() => setPanelOpen((visible) => !visible)} />
    {panelOpen && <InteractionPanel onClose={() => setPanelOpen(false)} onSettings={()=>setSettingsOpen(true)} />}
    {settingsOpen && <SettingsView onClose={()=>setSettingsOpen(false)} />}
  </main>;
}
