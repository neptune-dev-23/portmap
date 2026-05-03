import { Store } from "@tauri-apps/plugin-store";
import type { PortRange, RefreshPreset } from "./types";
import { REFRESH_PRESETS } from "./types";

let tauriStore: Store | null = null;

async function getStore(): Promise<Store> {
  if (!tauriStore) {
    tauriStore = await Store.load("config.json");
  }
  return tauriStore;
}

export const portRange = $state<PortRange>({ min: 3000, max: 9999 });
export const refreshIntervalMs = $state<{ value: number }>({ value: 1500 });
export const refreshPreset = $state<{ value: RefreshPreset }>({ value: "medium" });
export const zoomLevel = $state<{ value: number }>({ value: 1.0 });

export async function initStore() {
  const store = await getStore();

  const savedRange = await store.get<PortRange>("portRange");
  if (savedRange) {
    portRange.min = savedRange.min;
    portRange.max = savedRange.max;
  }

  const savedInterval = await store.get<number>("refreshIntervalMs");
  if (savedInterval) {
    refreshIntervalMs.value = savedInterval;
    refreshPreset.value = intervalToPreset(savedInterval);
  }

  const savedZoom = await store.get<number>("zoomLevel");
  if (savedZoom) {
    zoomLevel.value = savedZoom;
  }
}

export async function savePortRange(min: number, max: number) {
  portRange.min = min;
  portRange.max = max;
  const store = await getStore();
  await store.set("portRange", { min, max });
  await store.save();
}

export async function saveRefreshInterval(ms: number, preset: RefreshPreset) {
  refreshIntervalMs.value = ms;
  refreshPreset.value = preset;
  const store = await getStore();
  await store.set("refreshIntervalMs", ms);
  await store.save();
}

export async function saveZoomLevel(zoom: number) {
  zoomLevel.value = zoom;
  const store = await getStore();
  await store.set("zoomLevel", zoom);
  await store.save();
}

function intervalToPreset(ms: number): RefreshPreset {
  for (const [preset, val] of Object.entries(REFRESH_PRESETS)) {
    if (val === ms) return preset as RefreshPreset;
  }
  return "custom";
}
