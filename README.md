# Kai Desktop

## Island visual identity and motion

The primary surface now uses a small creature-led capsule with a graphite/sage material. Kai's existing layered SVG and internal CSS animations are preserved. Pointer movement inside the surface adds a bounded face glance; no global mouse listener is used. Expanded home offers a greeting using the optional local display name under More → Personalize, an integrated command field, and filename suggestions. Searches still use the existing local service; the field does not enable an AI provider.

Native presentation dimensions are logical Windows pixels: micro 68×52, compact 88×68, expanded 400×248, content/settings 400×430. A temporary stretch stage is 400×68. At idle the surface is fully transparent and only animated Kai is visible. Tauri converts logical sizing for DPI and centers using the actual physical outer width. Sleeping selects micro when collapsed. Escape or clicking Kai collapses; More contains personalization and temporary hide.

Opening stretches the capsule for 240 ms, then unfolds the contextual surface for 180 ms. Closing dismisses content for 120 ms, shrinks vertically, then closes the horizontal capsule for 240 ms. Native bounds change at each stage; there is no permanent oversized overlay. During each short reveal, clipping can leave transparent pixels within that stage's native rectangle; that temporary region still receives hit tests. Motion tokens are in `src/animations/motion.ts`, material and staged motion in `src/animations/island.css`. CSS handles animation; JavaScript only sequences finite native resize steps. Reduced Motion skips stage delays and disables motion, while preserving the character and layout. State surface accents run a limited number of times.

Visual quality and monitor/DPI behavior require manual Windows review. Foreground-monitor tracking remains unchanged. No claim of visual acceptance follows from compilation alone.

Kai is a lightweight, offline-first Windows desktop companion built with Tauri 2, Rust, React, TypeScript, and Vite. V1.1 adds an optional OpenAI-powered conversational brain and tightly scoped local tools while preserving the Island, animated character, local commands, search, system information, settings/memory, and tray controls.

## Development

```powershell
npm install
npm run tauri dev
```

Create a distributable Windows installer with `npm run tauri build`.

## Architecture

- `src/companion/KaiAnimatedCharacter.tsx` owns the layered character renderer; CSS animation is independent of event/state logic.
- `src/state/companion-store.tsx` is the central state manager. Integration code should call its `setState(state, source)` method through an event adapter; it must not manipulate character animation directly.
- `src/types/companion.ts` defines the supported state union and a small event/interface contract for future modules.
- `src/types/providers.ts` defines future AI tool and Codex activity boundaries.
- `src-tauri/src/main.rs` provides validated settings, search, memory, startup, system, file, and allowlisted app commands plus tray behavior.
- `src-tauri/src/credentials.rs` is the only API-key storage boundary and uses Windows Credential Manager under `Kai Desktop / OpenAI API`.
- `src-tauri/src/ai.rs` owns Responses API requests, bounded in-memory context, personality, tool definitions/execution, and provider error mapping.

## OpenAI brain

Online AI is opt-in. Settings lets the user securely save, replace, remove, and test an OpenAI API key, enable online answers, and choose GPT-5.6 Luna (default), Terra, or Sol. The key never crosses into the webview and is never written to Kai's JSON settings or logs. Every Responses API request sets `store: false`; this is not presented as a zero-retention guarantee.

Deterministic requests always bypass AI: filename searches, approved app launches, known-folder opening, connectivity, and system snapshots remain local. Other requests use OpenAI only when the feature is enabled, a key exists, and Windows reports connectivity. Conversation context is memory-only, capped at 16 messages, and can be cleared from Settings.

The model can request only `search_files`, `get_file_metadata`, `open_file`, `reveal_file`, `open_known_app`, and `get_system_info`. File contents are never read. Model-visible search data omits full paths and uses per-request numeric references; open/reveal resolves those references against the current local result set. There is no shell, arbitrary execution, destructive file operation, or model-supplied path.

V1.1 uses reliable non-streaming requests with a 45-second timeout. In-flight request cancellation is not yet exposed.

## State system

Supported states are `idle`, `working`, `thinking`, `success`, `error`, `sleeping`, and the search presentation. The layered face, eyes, eyelids, antenna, arms, chest light, body, and sleep indicators animate internally through CSS without a JavaScript frame loop.

## Future integrations

Future modules such as system monitoring, Codex monitoring, notifications, AI providers, Spotify, and Mobi Biz should implement `CompanionIntegration` and dispatch typed `CompanionEvent` values. They should receive only their minimum required permissions and call explicit backend commands—not arbitrary shell or filesystem APIs.

## Security model

The frontend capability applies only to the Island window. It exposes explicit Rust commands and window hiding; there is no shell, generic process, PowerShell, CMD, destructive file action, or arbitrary path mutation. User strings are bounded and validated. Known apps are mapped server-side. External and future AI content is data, never executable instructions.

## Known limitations

- Direct Everything SDK IPC, fullscreen detection, foreground-monitor following, installed-app discovery, and Codex activity ingestion are deferred as detailed below.

## Phase 2: offline-first platform

Kai Island is the only application window. It is placed at the top center of its current or primary monitor, stays transparent at idle, and materializes a contextual surface only during interaction.

The central event bus covers state, search, task, and connectivity events so integrations publish typed events rather than driving UI directly. Music, Download, and Codex presentation types remain extension points only.

### Local commands and search

The Rust-side local search service validates queries and performs bounded, on-demand filename searches only under the current user's Desktop, Documents, and Downloads folders. It never performs a continuous index or scans every drive. Results include name, path, extension, size, and directory status. File actions are read-only: open and reveal only. The frontend cannot crawl filesystems.

Everything is detected in its usual local installation locations. Kai implements the documented localhost HTTP JSON provider with path, size, and modified-time columns. Enable Everything's HTTP server to activate whole-PC search; no file downloads or remote access are needed. If the server is disabled, unreachable, or returns invalid data, Kai immediately uses its bounded fallback. Kai never installs or reconfigures Everything.

The allowlisted launcher supports Calculator, Notepad, File Explorer, and Windows Settings. There is no generic command execution interface. Lightweight disk-free-space information is available by request; connectivity is event-led, not ping-polled.

### Settings, privacy, and always-on behavior

Settings persist locally and cover display name, Kai visibility, fullscreen preference, reduced motion, startup, provider selection, and approved extra roots. Start with Windows uses the current user's `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` entry, requires no administrator rights, avoids duplicates, and removes the same named entry when disabled. The stored setting is reconciled with the registry.

Search identifies paths; it does not read file contents. Direct local searches stay on the PC. For an online tool-assisted search, Kai sends only the bounded result metadata needed to answer and withholds full paths and file contents. Model output can invoke only explicit registered capabilities and can never become a shell command. Kai has no permanent microphone recording or background network polling.

### Phase 2 limitations

- The installed Everything 1.4 application has IPC available, but its SDK DLL and `es.exe` are not installed. Kai uses Everything's documented HTTP interface when the user enables it; direct SDK IPC is deferred rather than bundling an unreviewed native DLL.
- Fullscreen detection, foreground-monitor following, installed-app discovery, Codex activity, and a fully local AI provider are deferred. The current monitor remains stable to avoid disruptive jumps.

`src/types/providers.ts` defines the AI tool and explicit Codex activity boundaries. AI requests receive only named Kai tools; the boundary contains no shell, PowerShell, CMD, or generic process capability. Codex has no stable public local activity source in this project, so only an explicit publisher interface is defined and no private data is scraped.

## Offline commands

The command field handles `find …`, `search for …`, MP4/JPG searches, Calculator, Notepad, File Explorer, Windows Settings, Downloads, Documents, storage, system info, battery, memory, and connectivity. Unknown requests get a short capability explanation. Searches and commands remain local.

System snapshots include CPU usage, used/total RAM, free/total disk space, Windows battery percentage/charging state when present, and Windows connectivity state. They are collected only after a user request.

Recent commands and search terms are saved in `memory.json` under Kai's per-user application data folder, capped at 20 of each, and can be cleared in Settings. File contents, API credentials, and full result histories are not stored.

## Windows manual test plan

- Launch with `npm run tauri dev`; confirm only transparent animated Kai appears top-center.
- Click Kai; confirm material stretch/unfold and collapse restore the 88×68 idle bounds.
- Enable Everything HTTP Server, search `*.jpg` and `Mobi Biz`, and confirm Settings reports Everything HTTP.
- Disable Everything HTTP Server, repeat a search, and confirm fallback results from the approved roots.
- Open and reveal a result; confirm there are no destructive actions.
- Try `open Calculator`, `open Downloads`, `show storage`, `memory usage`, `battery`, and `am I offline`.
- Toggle Start with Windows, inspect the current-user Startup Apps state, restart Windows, then disable it.
- Set a display name, reduced motion, extra fallback root, and restart Kai to verify persistence.
- Clear local history and confirm counts return to zero.
- Disconnect networking and confirm local commands continue; reconnect and confirm connectivity status changes on request.
- Test compact/expanded placement at 100%, 125%, and 150% DPI and after disconnecting a monitor.
- Test a fullscreen video/game manually; automatic fullscreen hiding is not implemented.
- Quit from the tray and relaunch Kai.
- Island monitor selection uses the Island's current monitor or primary monitor; automatic foreground-window monitor tracking is future work.
