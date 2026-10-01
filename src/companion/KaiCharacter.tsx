import { useEffect, useRef } from "react";
import kaiAsset from "../assets/kai-placeholder.png";
import { useCompanion } from "../state/companion-store";
import { beginDrag, persistWindowPosition } from "../services/window-service";

interface Props { onActivate: () => void; }

export function KaiCharacter({ onActivate }: Props) {
  const { state } = useCompanion();
  const dragging = useRef(false);
  const down = useRef({ x: 0, y: 0 });

  useEffect(() => {
    const timeout = state === "success" || state === "error"
      ? window.setTimeout(() => undefined, 1300)
      : undefined;
    return () => {
      if (timeout !== undefined) window.clearTimeout(timeout);
    };
  }, [state]);

  async function onPointerDown(event: React.PointerEvent) {
    if (event.button !== 0) return;
    down.current = { x: event.clientX, y: event.clientY };
    dragging.current = false;
  }
  async function onPointerMove(event: React.PointerEvent) {
    if (event.buttons !== 1 || dragging.current) return;
    if (Math.hypot(event.clientX - down.current.x, event.clientY - down.current.y) < 4) return;
    dragging.current = true;
    await beginDrag();
  }
  async function onPointerUp() {
    if (dragging.current) await persistWindowPosition();
    else onActivate();
  }

  return <button className={`kai-character state-${state}`} aria-label="Kai companion" onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={onPointerUp}>
    <img src={kaiAsset} draggable={false} alt="Kai, a small teal companion robot" />
    {state === "thinking" && <span className="thought">•••</span>}
    {state === "sleeping" && <span className="sleep">z z</span>}
  </button>;
}
