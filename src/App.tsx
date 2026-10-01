import { useEffect, useState } from "react";
import { KaiCharacter } from "./companion/KaiCharacter";
import { InteractionPanel } from "./components/InteractionPanel";
import { restoreWindowPosition } from "./services/window-service";

export default function App() {
  const [panelOpen, setPanelOpen] = useState(false);
  useEffect(() => { void restoreWindowPosition(); }, []);
  return <main className={panelOpen ? "app panel-open" : "app"} onContextMenu={(event) => { event.preventDefault(); setPanelOpen(true); }}>
    <KaiCharacter onActivate={() => setPanelOpen((visible) => !visible)} />
    {panelOpen && <InteractionPanel onClose={() => setPanelOpen(false)} />}
  </main>;
}
