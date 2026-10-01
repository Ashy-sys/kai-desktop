# Kai Desktop

## Island visual identity and motion

The primary surface now uses a small creature-led capsule with a graphite/sage material. Kai's existing layered SVG and internal CSS animations are preserved. Pointer movement inside the surface adds a bounded face glance; no global mouse listener is used. Expanded home offers a greeting using the optional local display name under More → Personalize, an integrated command field, and filename suggestions. Searches still use the existing local service; the field does not enable an AI provider.

Native presentation dimensions are logical Windows pixels: micro 88×48, compact 132×64, expanded 400×248, content/settings 400×430. A temporary stretch stage is 400×64. Tauri converts logical sizing for DPI and centers using the actual physical outer width. Sleeping selects micro when collapsed. Escape or clicking Kai collapses; More contains personalization and temporary hide.

Opening stretches the capsule for 240 ms, then unfolds the contextual surface for 180 ms. Closing dismisses content for 120 ms, shrinks vertically, then closes the horizontal capsule for 240 ms. Native bounds change at each stage; there is no permanent oversized overlay. During each short reveal, clipping can leave transparent pixels within that stage's native rectangle; that temporary region still receives hit tests. Motion tokens are in `src/animations/motion.ts`, material and staged motion in `src/animations/island.css`. CSS handles animation; JavaScript only sequences finite native resize steps. Reduced Motion skips stage delays and disables motion, while preserving the character and layout. State surface accents run a limited number of times.

Visual quality and monitor/DPI behavior require manual Windows review. Foreground-monitor tracking remains unchanged. No claim of visual acceptance follows from compilation alone.

Kai is a lightweight Windows desktop companion built with Tauri 2, Rust, React, TypeScript, and Vite. Phase 1 provides a borderless transparent companion window, a generated original placeholder character, state-driven animation, a small interaction panel, a system tray menu, and persisted placement.

## Development

```powershell
npm install
npm run tauri dev
```

Create a distributable Windows installer with `npm run tauri build`.

## Architecture

- `src/companion/` owns the character view and drag interaction.
- `src/state/companion-store.tsx` is the central state manager. Integration code should call its `setState(state, source)` method through an event adapter; it must not manipulate character animation directly.
- `src/types/companion.ts` defines the supported state union and a small event/interface contract for future modules.
- `src/services/` holds narrow Tauri-facing services. The frontend never runs shell commands.
- `src/assets/kai-placeholder.png` is the replaceable, original development character asset.
- `src-tauri/src/main.rs` provides the allowlisted commands for position persistence and explicit quit, plus tray behavior and lightweight logs.

## State system

Supported states are `idle`, `working`, `thinking`, `success`, `error`, and `sleeping`. The interaction panel contains developer controls for each state. CSS presently maps those states to lightweight transforms; later sprite or animation assets can replace that mapping without changing consumers.

## Future integrations

Future modules such as system monitoring, Codex monitoring, notifications, AI providers, Spotify, and Mobi Biz should implement `CompanionIntegration` and dispatch typed `CompanionEvent` values. They should receive only their minimum required permissions and call explicit backend commands—not arbitrary shell or filesystem APIs.

## Security model

The frontend has a restricted capability set and invokes only three explicit Rust commands: save/load position and quit. No shell execution is exposed. Do not commit credentials; future credentials belong in secure storage. Treat external and AI-provided content as data, never executable instructions.

## Known limitations

- Windows click-through transparency is intentionally deferred: broad click-through would make Kai difficult to interact with and needs a carefully scoped native implementation.
- The saved position uses physical pixels and is discarded if it is outside every connected monitor's visible margin; restoring a position across dramatically different DPI layouts may still need refinement.
- The Settings tray item currently reveals Kai; a settings view is a future phase feature.

## Phase 2: offline-first platform

Kai Island is a second transparent Tauri window, placed at the top center of the active/primary monitor. It shares the same `KaiEvent` runtime as the companion, collapses into an original small Kai pill, and expands into a compact local search surface. It does not emulate another product's visuals or interaction design.

The central event bus covers state, search, task, and connectivity events. Both UI windows listen to the same Tauri event stream, so integrations publish typed events rather than driving UI directly. The Island includes developer state events; its Music, Download, and Codex presentation types remain extension points only.

### Local commands and search

The Rust-side local search service validates queries and performs bounded, on-demand filename searches only under the current user's Desktop, Documents, and Downloads folders. It never performs a continuous index or scans every drive. Results include name, path, extension, size, and directory status. File actions are read-only: open and reveal only. The frontend cannot crawl filesystems.

Everything is detected by checking its usual local installation locations. Kai never installs it and does not claim an IPC connection until a documented local Everything interface is explicitly configured. Until then the safe fallback is used.

The allowlisted launcher supports Calculator, Notepad, File Explorer, and Windows Settings. There is no generic command execution interface. Lightweight disk-free-space information is available by request; connectivity is event-led, not ping-polled.

### Settings, privacy, and always-on behavior

Settings persist locally and currently cover companion/Island visibility, Island visibility policy, fullscreen preference, reduced motion, startup preference, and search-provider preference. Windows start-at-login is intentionally persisted as a preference only; registry/task startup registration is deferred until it has an explicit user-facing consent flow.

Search identifies paths; it does not read file contents. No path, filename, metadata, content, or index is sent to an AI provider. Future AI providers must use explicit registered capabilities and secure credential storage; they can never turn model output into shell commands. Kai has no permanent microphone recording or background network polling.

### Phase 2 limitations

- Everything detection is implemented; Everything IPC/CLI invocation is deliberately deferred pending a documented, version-safe adapter.
- Fullscreen detection, actual Windows startup registration, installed-app discovery, CPU/RAM/battery detail, and local/online AI providers are deferred.
- Island monitor selection uses the Island's current monitor or primary monitor; automatic foreground-window monitor tracking is future work.
