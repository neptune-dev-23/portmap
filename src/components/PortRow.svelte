<script lang="ts">
  import { killProcess, copyUrl, copyPath } from "../lib/tauri";
  import type { PortEntry } from "../lib/types";

  let { entry }: { entry: PortEntry } = $props();

  let killing = $state(false);
  let copied = $state(false);
  let copiedPath = $state(false);

  function formatCpu(value: number | null): string {
    return value === null ? "—" : `${value.toFixed(1)}%`;
  }

  function formatMemory(bytes: number | null): string {
    if (bytes === null) return "—";
    const mib = bytes / 1024 ** 2;
    return mib >= 1024 ? `${(mib / 1024).toFixed(2)} GiB` : `${mib.toFixed(1)} MiB`;
  }

  function coverage(seconds: number): string {
    return seconds < 60 ? `${Math.floor(seconds)}s` : `${Math.floor(seconds / 60)}m`;
  }

  async function handleCopyPath() {
    if (!entry.cwd) return;
    await copyPath(entry.cwd);
    copiedPath = true;
    setTimeout(() => (copiedPath = false), 1500);
  }

  async function handleKill() {
    killing = true;
    try {
      await killProcess(entry.pid);
    } catch (e) {
      console.error("kill failed:", e);
    } finally {
      killing = false;
    }
  }

  async function handleCopy() {
    await copyUrl(entry.port);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }
</script>

<tr>
  <td class="port">{entry.port}</td>
  <td class="process" title={entry.executable_path ?? entry.process_name}>{entry.process_name}</td>
  <td class="pid">{entry.pid}</td>
  <td class="project">{entry.project_name ?? "—"}</td>
  <td class="metric" title={entry.cpu_percent === null ? "Waiting for two resource samples, or process access unavailable" : "Latest sample; 100% is one CPU core"}>
    {formatCpu(entry.cpu_percent)}
  </td>
  <td class="metric" title={entry.cpu_average_percent === null ? "Waiting for two resource samples, or process access unavailable" : `Average over ${coverage(entry.cpu_history_seconds)} of observed history; up to 1 hour`}>
    {formatCpu(entry.cpu_average_percent)}
    {#if entry.cpu_average_percent !== null && entry.cpu_history_seconds < 3600}
      <span class="coverage">over {coverage(entry.cpu_history_seconds)}</span>
    {/if}
  </td>
  <td class="metric" title="Current physical memory footprint; — means unavailable">{formatMemory(entry.memory_bytes)}</td>
  <td class="metric" title="Kernel estimate of CPU power; excludes GPU and other system energy. — means warming up or unsupported.">
    {entry.power_watts === null ? "—" : `${entry.power_watts.toFixed(2)} W`}
  </td>
  <td class="actions">
    <button class="btn-copy" onclick={handleCopy} title="Copy localhost URL">
      {copied ? "✓" : "Copy URL"}
    </button>
    {#if entry.cwd}
      <button class="btn-path" onclick={handleCopyPath} title="Copy shell-escaped path (cd-ready)">
        {copiedPath ? "✓" : "Copy Path"}
      </button>
    {/if}
    <button class="btn-kill" onclick={handleKill} disabled={killing} title="Send SIGTERM">
      {killing ? "…" : "Kill"}
    </button>
  </td>
</tr>

<style>
tr {
  border-bottom: 1px solid var(--border);
  transition: background 0.1s;
}
tr:hover {
  background: var(--row-hover);
}
td {
  padding: 8px 10px;
  font-size: 13px;
  white-space: nowrap;
}
.port {
  font-weight: 600;
  color: var(--accent);
  font-variant-numeric: tabular-nums;
  width: 60px;
}
.pid {
  color: var(--muted);
  font-variant-numeric: tabular-nums;
  width: 60px;
}
.process {
  font-family: monospace;
  font-size: 12px;
  white-space: normal;
  overflow-wrap: anywhere;
  min-width: 150px;
}
.metric {
  font-variant-numeric: tabular-nums;
}
.coverage {
  display: block;
  margin-top: 2px;
  font-size: 10px;
  color: var(--muted);
}
.project {
  color: var(--text-secondary);
}
.actions {
  display: flex;
  gap: 6px;
  align-items: center;
}
button {
  padding: 3px 8px;
  border-radius: 5px;
  border: 1px solid var(--border);
  font-size: 11px;
  cursor: pointer;
  background: var(--btn-bg);
  color: var(--text);
  transition: background 0.1s, border-color 0.1s;
}
button:hover:not(:disabled) {
  border-color: var(--accent);
}
button:disabled {
  opacity: 0.5;
  cursor: default;
}
.btn-kill {
  color: var(--danger);
  border-color: var(--danger-border);
}
.btn-kill:hover:not(:disabled) {
  background: var(--danger-bg);
  border-color: var(--danger);
}
.btn-copy {
  min-width: 62px;
}
.btn-path {
  min-width: 72px;
}
</style>
