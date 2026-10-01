import { companionStates } from "../types/companion";
import { useCompanion } from "../state/companion-store";
import { quitKai } from "../services/window-service";

interface Props { onClose: () => void; onSettings: () => void; }
export function InteractionPanel({ onClose, onSettings }: Props) {
  const { state, setState } = useCompanion();
  return <aside className="interaction-panel" aria-label="Kai controls">
    <div className="panel-top"><div><strong>Kai</strong><span>Your desktop companion</span></div><button className="close" onClick={onClose} aria-label="Close panel">×</button></div>
    <p>Hi! I’m keeping you company.</p>
    <div className="state-label"><span>Current state</span><b>{state}</b></div>
    <div className="state-grid">{companionStates.map((item) => <button key={item} className={item === state ? "active" : ""} onClick={() => setState(item, "developer-controls")}>{item}</button>)}</div>
    <div className="panel-actions"><button onClick={onSettings}>Settings</button><button className="quit" onClick={() => void quitKai()}>Quit</button></div>
  </aside>;
}
