# Kai Desktop

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
