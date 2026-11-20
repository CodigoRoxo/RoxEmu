import { invoke, isTauri } from "@tauri-apps/api/core";

export interface AppInfo {
  version: string;
  data: { root: string; bios: string; portable: boolean };
  systems: { id: string; name: string; extensions: string[]; default_core: string; requires_bios: boolean }[];
}

/** Informações do backend Rust. Fora do Tauri (ex.: `pnpm vite` no navegador) devolve `null`. */
export async function getAppInfo(): Promise<AppInfo | null> {
  if (!isTauri()) return null;
  return invoke<AppInfo>("app_info");
}
