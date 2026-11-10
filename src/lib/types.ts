export interface PortEntry {
  port: number;
  pid: number;
  process_name: string;
  executable_path: string | null;
  project_name: string | null;
  cwd: string | null;
  cpu_percent: number | null;
  cpu_average_percent: number | null;
  cpu_history_seconds: number;
  memory_bytes: number | null;
  power_watts: number | null;
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
