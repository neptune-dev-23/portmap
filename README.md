# PortMap

A lightweight macOS utility that shows what's listening on your dev ports — and lets you act on it instantly.

<img src="static/logo.svg" alt="PortMap" width="160">

## Features

- **Live port list** — polls TCP listeners in a configurable range (default 3000–9999) every 1.5 s
- **Project inference** — resolves each process's working directory and walks up to find the nearest `package.json`, `Cargo.toml`, or `pyproject.toml` to show the project name
- **Copy URL** — copies `http://localhost:PORT` to clipboard
- **Copy Path** — copies the project directory as a shell-escaped path, ready to `cd` into
- **Kill** — sends SIGTERM to the process
- **Configurable refresh** — Fast (500 ms), Medium (1.5 s), Slow (5 s), or Custom
- **Zoom** — Cmd+= / Cmd+- / Cmd+0, persisted across launches
- **Window size** — remembered across launches

## Stack

- [Tauri v2](https://tauri.app) + Rust
- [Svelte 5](https://svelte.dev) + TypeScript
- Bun

## Development

```bash
bun install
bun run tauri dev
```

## Build

```bash
# Full build (includes .app + .dmg installer)
bun run tauri build

# .app only — skips DMG creation
bun run tauri build --bundles app
```

Output: `src-tauri/target/release/bundle/macos/PortMap.app`

Drag into `/Applications`. On first launch, right-click → Open to bypass Gatekeeper (app is unsigned).

> **Note:** DMG creation requires Automation permission granted to your terminal in **System Settings → Privacy & Security → Automation → Finder**.

## Attribution

Built with [Claude Sonnet 4.6](https://anthropic.com) and [Claude Code](https://claude.ai/code).
