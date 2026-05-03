import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { PortEntry, PortRange } from "./types";

export const listPorts = (range: PortRange): Promise<PortEntry[]> =>
  invoke("list_ports", { minPort: range.min, maxPort: range.max });

export const killProcess = (pid: number): Promise<void> =>
  invoke("kill_process", { pid });

export const copyUrl = (port: number): Promise<void> =>
  writeText(`http://localhost:${port}`);

export const copyPath = (path: string): Promise<void> =>
  writeText(shellEscape(path));

function shellEscape(path: string): string {
  // Single-quote the path; escape any embedded single quotes as '\''
  return "'" + path.replace(/'/g, "'\\''") + "'";
}
