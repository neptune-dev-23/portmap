<script lang="ts">
  import { listPorts } from "../lib/tauri";
  import { portRange, refreshIntervalMs } from "../lib/store.svelte";
  import PortRow from "./PortRow.svelte";
  import type { PortEntry } from "../lib/types";

  let ports = $state<PortEntry[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(true);

  async function refresh() {
    try {
      ports = await listPorts({ min: portRange.min, max: portRange.max });
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const interval = refreshIntervalMs.value;
    const range = { min: portRange.min, max: portRange.max };
    void range; // track dependency
    refresh();
    const id = setInterval(refresh, interval);
    return () => clearInterval(id);
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
    <table>
      <thead>
        <tr>
          <th>Port</th>
          <th>Process</th>
          <th>PID</th>
          <th>Project</th>
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
