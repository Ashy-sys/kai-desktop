import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { attachConsole, info } from "@tauri-apps/plugin-log";
import App from "./App";
import { CompanionProvider } from "./state/companion-store";
import "./styles.css";

void attachConsole();
void info("Kai frontend started");
document.addEventListener("visibilitychange", () => { document.documentElement.dataset.hidden = String(document.hidden); });
createRoot(document.getElementById("root")!).render(<StrictMode><CompanionProvider><App /></CompanionProvider></StrictMode>);
