import { getCurrentWindow, PhysicalPosition } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { info, warn } from "@tauri-apps/plugin-log";

export async function beginDrag() {
  await getCurrentWindow().startDragging();
}

export async function persistWindowPosition() {
  try {
    const position = await getCurrentWindow().outerPosition();
    await invoke("save_window_position", { x: position.x, y: position.y });
    await info(`Saved Kai position: ${position.x}, ${position.y}`);
  } catch (error) {
    await warn(`Could not persist Kai position: ${String(error)}`);
  }
}

export async function restoreWindowPosition() {
  const saved = await invoke<{ x: number; y: number } | null>("load_window_position");
  if (saved) await getCurrentWindow().setPosition(new PhysicalPosition(saved.x, saved.y));
}

export async function quitKai() {
  await invoke("quit_app");
}
