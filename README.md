# PortMap

A lightweight macOS utility that shows what's listening on your dev ports — and lets you act on it instantly.

<img src="static/logo.svg" alt="PortMap" width="160">

## Features

- **Live port list** — polls TCP listeners in a configurable range (default 3000–9999) every 1.5 s
- **Project inference** — resolves each process's working directory and walks up to find the nearest `package.json`, `Cargo.toml`, or `pyproject.toml` to show the project name
- **Full process names** — resolves the executable through macOS `libproc`; hover over its name for the executable path
- **Lightweight resource usage** — current CPU, rolling one-hour average CPU, RAM (physical footprint), and estimated CPU power where supported, sampled per unique listening PID at most every 15 s
- **Copy URL** — copies `http://localhost:PORT` to clipboard
- **Copy Path** — copies the project directory as a shell-escaped path, ready to `cd` into
- **Kill** — sends SIGTERM to the process
- **Configurable refresh** — Fast (500 ms), Medium (1.5 s), Slow (5 s), or Custom
- **Zoom** — Cmd+= / Cmd+- / Cmd+0, persisted across launches
- **Window size** — remembered across launches

Resource metrics cover the listening process itself, excluding child processes.
CPU uses the macOS convention of 100% per core, so a multithreaded process can
exceed 100%. The one-hour average is time-weighted from cumulative CPU counters;
until an hour is available, each row shows its actual history duration. The
window boundary is interpolated between samples. RAM includes compressed/private
memory and may differ from RSS. A dash means warming up or access unavailable.

Port discovery uses one `lsof` subprocess per refresh with machine-readable,
untruncated output. Executable paths, working directories, and resource counters
come from direct `libproc` calls, cached for 15 s (or longer at slower custom
refresh rates). Multiple ports owned by one process share a sample and history.
History is kept only in memory, capped at 242 samples per in-range listening PID,
and discarded when that PID leaves the list or PortMap closes. PID reuse resets
the history. Polling pauses while the document is hidden; cumulative counters
can bridge shorter pauses, but gaps longer than an hour restart history.

On supported Apple Silicon Macs, CPU power is the change in the kernel's
`RUSAGE_INFO_V6.ri_energy_nj` estimate divided by elapsed time, displayed in watts.
This uses the same resource query as CPU/RAM. Older kernels fall back to V0;
unsupported, inaccessible, or all-zero energy counters show a dash. CPU counters
are converted from Mach ticks using `mach_timebase_info` before calculating CPU
percentages.

CPU power excludes GPU, display, networking, and other system energy; it does not
measure total app battery drain. Per-process GPU utilization and battery drain
are not reported because consistent attribution for arbitrary processes would
require more intrusive or privileged collection.

## Stack

- [Tauri v2](https://tauri.app) + Rust
- [Svelte 5](https://svelte.dev) + TypeScript
- pnpm (version pinned in `package.json`)

## Development

Requires macOS, Rust with Cargo, Node.js 22.12+ (or 24+), and the pnpm version
specified in `package.json`. The optional routing service also requires Caddy
and Python 3. Frontend dependencies use `pnpm-lock.yaml`; keep it in sync with
`package.json` when updating packages.

```bash
pnpm install --frozen-lockfile
pnpm tauri dev
```

## Build

```bash
# Full build (includes .app + .dmg installer)
pnpm tauri build

# .app only — skips DMG creation
pnpm tauri build --bundles app
```

Output: `src-tauri/target/release/bundle/macos/PortMap.app`

Drag into `/Applications`. On first launch, right-click → Open to bypass Gatekeeper (app is unsigned).

> **Note:** DMG creation requires Automation permission granted to your terminal in **System Settings → Privacy & Security → Automation → Finder**.

## Checks

```bash
pnpm check
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --lib --tests -- -D warnings
bash -n scripts/install-proxy.sh
python3 service/test_proxy.py
```

Native tests run on macOS. The listener and proxy tests bind temporary loopback
ports and require permission to create local sockets; they do not install the
service or require administrator access. GitHub Actions runs these checks on
macOS, including Caddy routing tests.

## Automatic localhost routing

The optional Caddy service maps `http://PORT.localhost` to `http://localhost:PORT`
for HTTP and WebSocket services on ports 1–65535, excluding port 80 (the proxy
itself). No individual port registrations or DNS table changes are needed on
macOS. This service runs independently of the PortMap app; the app's existing
Copy URL action still copies the direct localhost URL.

```bash
brew install caddy
bash scripts/install-proxy.sh
```

Installation requests administrator authentication because this Mac restricts
port 80. It installs a launchd daemon, with root-owned copies of Caddy and its
configuration in `/Library/Application Support/PortMap/Proxy`. It starts at boot.
Run the installer again after changing the service files or upgrading Caddy.
The launcher uses `/usr/bin/python3` (provided by Apple's command line tools).

Every start explicitly binds `localhost`, `127.0.0.1`, and `[::1]`, plus this
machine's currently assigned Tailscale IPv4/IPv6 addresses from `tailscale ip`.
There are no wildcard listeners (`0.0.0.0` or `[::]`), automatic HTTPS listeners,
or Caddy admin API. If Tailscale is missing, unavailable, or its addresses are
not assigned yet, routing starts on loopback only. Tailscale discovery runs in
the logged-in console user's session; a boot before login also starts with
loopback only. Restart after login, after Tailscale connects, or its addresses change:

```bash
sudo launchctl kickstart -k system/com.portmap.proxy
sudo lsof -nP -iTCP:80 -sTCP:LISTEN
```

The Tailscale bindings allow devices permitted by your tailnet policy to proxy
to local HTTP services, including services whose own listeners are loopback
only. Other devices resolve `*.localhost` to themselves, so remote access also
requires a client-side hostname override. For example, replacing the address
below with this Mac's Tailscale IPv4 address:

```bash
curl --noproxy '*' --resolve 3000.localhost:80:100.x.y.z http://3000.localhost
```

Requests with unrelated hostnames, invalid port numbers, or port 80 return 404;
an unavailable upstream returns 502. Upstreams use plain HTTP, with the Host
header set to `localhost:PORT` for dev-server compatibility. Applications may
still require their own origin, callback URL, or WebSocket configuration.

Logs: `/var/log/portmap/proxy.log` and `/var/log/portmap/proxy-error.log`.
To stop and uninstall the service:

```bash
sudo launchctl bootout system/com.portmap.proxy
sudo rm /Library/LaunchDaemons/com.portmap.proxy.plist
sudo rm -r '/Library/Application Support/PortMap/Proxy'
```

To test routing, IPv4/IPv6 upstreams, WebSocket upgrades, invalid hosts, Tailscale
discovery, and the absence of wildcard listeners without administrator access:

```bash
python3 service/test_proxy.py
```

## Attribution

Built with [Claude Sonnet 4.6](https://anthropic.com) and [Claude Code](https://claude.ai/code).
