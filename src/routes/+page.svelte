<script lang="ts">
  import { onMount } from "svelte";
  import { initStore, zoomLevel, saveZoomLevel } from "../lib/store.svelte";
  import PortList from "../components/PortList.svelte";
  import ConfigPanel from "../components/ConfigPanel.svelte";

  const ZOOM_STEP = 0.1;
  const ZOOM_MIN = 0.5;
  const ZOOM_MAX = 2.5;

  let ready = $state(false);

  function applyZoom(zoom: number) {
    document.documentElement.style.zoom = String(zoom);
    saveZoomLevel(zoom);
  }

  onMount(() => {
    function onKeydown(e: KeyboardEvent) {
      if (!e.metaKey) return;
      if (e.key === "=" || e.key === "+") {
        e.preventDefault();
        applyZoom(Math.min(ZOOM_MAX, Math.round((zoomLevel.value + ZOOM_STEP) * 10) / 10));
      } else if (e.key === "-") {
        e.preventDefault();
        applyZoom(Math.max(ZOOM_MIN, Math.round((zoomLevel.value - ZOOM_STEP) * 10) / 10));
      } else if (e.key === "0") {
        e.preventDefault();
        applyZoom(1.0);
      }
    }

    window.addEventListener("keydown", onKeydown);

    initStore().then(() => {
      document.documentElement.style.zoom = String(zoomLevel.value);
      ready = true;
    });

    return () => window.removeEventListener("keydown", onKeydown);
  });
</script>

<div class="app">
  <header>
    <span class="logo">
      <img src="/logo.svg" alt="" width="22" height="22" />
      PortMap
    </span>
    <ConfigPanel />
    <span class="syncing" class:visible={!ready} aria-hidden="true"></span>
  </header>
  <PortList />
</div>

<style>
:global(*) {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

:global(:root) {
  --accent: #3b82f6;
  --border: #e5e7eb;
  --btn-bg: #ffffff;
  --text: #111827;
  --text-secondary: #374151;
  --muted: #9ca3af;
  --row-hover: #f9fafb;
  --danger: #ef4444;
  --danger-border: #fca5a5;
  --danger-bg: #fee2e2;
  --header-bg: #ffffff;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 14px;
  background: #f9fafb;
  color: var(--text);
}

@media (prefers-color-scheme: dark) {
  :global(:root) {
    --accent: #60a5fa;
    --border: #374151;
    --btn-bg: #1f2937;
    --text: #f9fafb;
    --text-secondary: #d1d5db;
    --muted: #6b7280;
    --row-hover: #1f2937;
    --danger: #f87171;
    --danger-border: #7f1d1d;
    --danger-bg: #450a0a;
    --header-bg: #111827;
    background: #111827;
    color: var(--text);
  }
}

.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  border-bottom: 1px solid var(--border);
  background: var(--header-bg);
  flex-shrink: 0;
  gap: 16px;
}

.logo {
  display: flex;
  align-items: center;
  gap: 7px;
  font-weight: 700;
  font-size: 15px;
  letter-spacing: -0.02em;
  color: var(--accent);
  white-space: nowrap;
}
.logo img {
  border-radius: 5px;
  flex-shrink: 0;
}

.syncing {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--muted);
  opacity: 0;
  transition: opacity 0.2s;
  flex-shrink: 0;
  align-self: center;
}
.syncing.visible {
  opacity: 1;
  animation: pulse 1s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 0.3; }
  50% { opacity: 1; }
}
</style>
