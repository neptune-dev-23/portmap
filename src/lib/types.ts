export interface PortEntry {
  port: number;
  pid: number;
  process_name: string;
  project_name: string | null;
  cwd: string | null;
}

export interface PortRange {
  min: number;
  max: number;
}

export type RefreshPreset = "fast" | "medium" | "slow" | "custom";

export const REFRESH_PRESETS: Record<Exclude<RefreshPreset, "custom">, number> = {
  fast: 500,
  medium: 1500,
  slow: 5000,
};
