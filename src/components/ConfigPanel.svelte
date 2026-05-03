<script lang="ts">
  import { portRange, refreshIntervalMs, refreshPreset, savePortRange, saveRefreshInterval } from "../lib/store.svelte";
  import type { RefreshPreset } from "../lib/types";
  import { REFRESH_PRESETS } from "../lib/types";

  let minInput = $state(portRange.min);
  let maxInput = $state(portRange.max);
  let customMs = $state(refreshIntervalMs.value);

  const presets: { label: string; value: RefreshPreset }[] = [
    { label: "Fast", value: "fast" },
    { label: "Medium", value: "medium" },
    { label: "Slow", value: "slow" },
    { label: "Custom", value: "custom" },
  ];

  function applyRange() {
    const min = Math.max(1, Math.min(minInput, 65534));
    const max = Math.max(min + 1, Math.min(maxInput, 65535));
    savePortRange(min, max);
    minInput = min;
    maxInput = max;
  }

  function selectPreset(preset: RefreshPreset) {
    if (preset !== "custom") {
      const ms = REFRESH_PRESETS[preset];
      customMs = ms;
      saveRefreshInterval(ms, preset);
    } else {
      saveRefreshInterval(customMs, "custom");
    }
  }

  function applyCustom() {
    const ms = Math.max(200, customMs);
    customMs = ms;
    saveRefreshInterval(ms, "custom");
  }
</script>

<div class="config">
  <div class="config-group">
    <span class="config-label">Port range</span>
    <div class="range-inputs">
      <input type="number" bind:value={minInput} min="1" max="65534" onchange={applyRange} />
      <span class="sep">–</span>
      <input type="number" bind:value={maxInput} min="2" max="65535" onchange={applyRange} />
    </div>
  </div>

  <div class="config-group">
    <span class="config-label">Refresh</span>
    <div class="presets">
      {#each presets as p}
        <button
          class="preset-btn"
          class:active={refreshPreset.value === p.value}
          onclick={() => selectPreset(p.value)}
        >
          {p.label}
        </button>
      {/each}
      {#if refreshPreset.value === "custom"}
        <input
          class="custom-ms"
          type="number"
          bind:value={customMs}
          min="200"
          placeholder="ms"
          onchange={applyCustom}
        />
      {/if}
    </div>
  </div>
</div>

<style>
.config {
  display: flex;
  gap: 24px;
  align-items: center;
  flex-wrap: wrap;
}
.config-group {
  display: flex;
  align-items: center;
  gap: 8px;
}
span.config-label {
  font-size: 12px;
  color: var(--muted);
  white-space: nowrap;
}
.range-inputs {
  display: flex;
  align-items: center;
  gap: 4px;
}
.range-inputs input {
  width: 70px;
  padding: 3px 6px;
  border-radius: 5px;
  border: 1px solid var(--border);
  background: var(--btn-bg);
  color: var(--text);
  font-size: 12px;
  text-align: center;
}
.sep {
  color: var(--muted);
  font-size: 12px;
}
.presets {
  display: flex;
  gap: 4px;
  align-items: center;
}
.preset-btn {
  padding: 3px 10px;
  border-radius: 5px;
  border: 1px solid var(--border);
  background: var(--btn-bg);
  color: var(--text);
  font-size: 12px;
  cursor: pointer;
  transition: background 0.1s, border-color 0.1s;
}
.preset-btn:hover {
  border-color: var(--accent);
}
.preset-btn.active {
  background: var(--accent);
  color: #fff;
  border-color: var(--accent);
}
.custom-ms {
  width: 70px;
  padding: 3px 6px;
  border-radius: 5px;
  border: 1px solid var(--border);
  background: var(--btn-bg);
  color: var(--text);
  font-size: 12px;
  text-align: center;
}
</style>
