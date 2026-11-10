<script lang="ts">
  import { listPorts } from "../lib/tauri";
  import { portRange, refreshIntervalMs } from "../lib/store.svelte";
  import PortRow from "./PortRow.svelte";
  import type { PortEntry } from "../lib/types";

  let ports = $state<PortEntry[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(true);

  $effect(() => {
    const interval = refreshIntervalMs.value;
    const range = { min: portRange.min, max: portRange.max };
    let disposed = false;
    let running = false;
    let timer: ReturnType<typeof setTimeout> | undefined;

    async function refresh() {
      if (disposed || running || document.hidden) return;
      running = true;
      try {
        const entries = await listPorts(range);
        if (!disposed) {
          ports = entries;
          error = null;
        }
      } catch (e) {
        if (!disposed) error = String(e);
      } finally {
        running = false;
        if (!disposed) {
          loading = false;
          // Wait for completion before scheduling; slow scans cannot pile up.
          if (!document.hidden) timer = setTimeout(refresh, interval);
        }
      }
    }

    function visibilityChanged() {
      clearTimeout(timer);
      if (!document.hidden) void refresh();
    }

    void refresh();
    document.addEventListener("visibilitychange", visibilityChanged);
    return () => {
      disposed = true;
      clearTimeout(timer);
      document.removeEventListener("visibilitychange", visibilityChanged);
    };
  });
</script>

<div class="port-list">
  {#if loading}
    <p class="status">Scanning ports…</p>
  {:else if error}
    <p class="status error">{error}</p>
  {:else if ports.length === 0}
    <p class="status muted">No listening ports in range {portRange.min}–{portRange.max}</p>
  {:else}
    <p class="metrics-note">
      Resources update every 15s or slower. CPU: 100% = one core. Per process; excludes children.
      History builds while open; up to 1h. RAM = physical footprint. Power = estimated CPU watts.
    </p>
    <table>
      <thead>
        <tr>
          <th>Port</th>
          <th>Process</th>
          <th>PID</th>
          <th>Project</th>
          <th title="CPU used during the latest sampling interval">CPU</th>
          <th title="Time-weighted CPU average over the last hour, or available history">CPU · 1h avg</th>
          <th title="Current process memory footprint, including compressed memory">RAM</th>
          <th title="Estimated CPU power from macOS energy counters on supported Apple Silicon Macs; excludes GPU and other system energy">CPU power</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        {#each ports as entry (entry.port + "-" + entry.pid)}
          <PortRow {entry} />
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
.port-list {
  flex: 1;
  overflow: auto;
}
.status {
  padding: 32px;
  text-align: center;
  font-size: 13px;
}
.status.error {
  color: var(--danger);
}
.status.muted {
  color: var(--muted);
}
.metrics-note {
  padding: 8px 10px;
  font-size: 11px;
  color: var(--muted);
}
table {
  width: 100%;
  border-collapse: collapse;
}
thead tr {
  border-bottom: 2px solid var(--border);
}
th {
  padding: 8px 10px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--muted);
  text-align: left;
}
</style>
