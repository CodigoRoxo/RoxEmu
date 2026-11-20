import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface AppInfo {
  version: string;
  data: { root: string; bios: string; portable: boolean };
  ps1_bios: boolean;
  systems: { id: string; name: string; extensions: string[]; default_core: string; requires_bios: boolean }[];
}

/** Jogo como o backend guarda (library.json). */
export interface LibraryGame {
  id: number;
  system: "ps1" | "snes" | "gba";
  title: string;
  region: string | null;
  serial: string | null;
  format: string;
  discs: number;
  size_bytes: number;
  path: string;
  added_at: number;
  last_played: number | null;
  playtime_secs: number;
  favorite: boolean;
  cover: string | null;
}

export type RunnerEvent =
  | { kind: "started"; game_id: number }
  | { kind: "exited"; game_id: number; played_secs: number }
  | { kind: "error"; game_id: number; message: string };

/** `true` quando roda dentro do app (e não no navegador com `pnpm vite`). */
export const inApp = isTauri();

export async function getAppInfo(): Promise<AppInfo | null> {
  return inApp ? invoke<AppInfo>("app_info") : null;
}

export async function listGames(): Promise<LibraryGame[]> {
  return inApp ? invoke<LibraryGame[]>("list_games") : [];
}

/** Abre o seletor de arquivos. `null` se o usuário cancelar. */
export async function addGame(): Promise<LibraryGame | null> {
  if (!inApp) throw new Error("Adicionar jogos só funciona dentro do app ROXemu.");
  return invoke<LibraryGame | null>("add_game");
}

export const removeGame = (id: number) => invoke<void>("remove_game", { id });
export const setFavorite = (id: number, favorite: boolean) => invoke<void>("set_favorite", { id, favorite });
export const launchGame = (id: number) => invoke<void>("launch_game", { id });
export const openBiosFolder = () => invoke<void>("open_bios_folder");

export const refreshCover = (id: number) => invoke<void>("refresh_cover", { id });
/** Escolher imagem própria como capa. `false` se cancelar. */
export const chooseCover = (id: number) => invoke<boolean>("choose_cover", { id });

/** Avisado quando algo muda na biblioteca em segundo plano (ex.: capa baixada). */
export function onLibraryChanged(cb: () => void): Promise<UnlistenFn> {
  if (!inApp) return Promise.resolve(() => {});
  return listen("library-changed", () => cb());
}

/** Avisado quando a busca de capa de um jogo não encontrou nada (ou falhou). */
export function onCoverMissing(cb: (e: { title: string; error: string | null }) => void): Promise<UnlistenFn> {
  if (!inApp) return Promise.resolve(() => {});
  return listen<{ title: string; error: string | null }>("cover-missing", (e) => cb(e.payload));
}

/** URL que a interface usa para mostrar um arquivo de capa do disco. */
export function fileUrl(path: string): string {
  return convertFileSrc(path);
}

export function onRunnerEvent(cb: (e: RunnerEvent) => void): Promise<UnlistenFn> {
  if (!inApp) return Promise.resolve(() => {});
  return listen<RunnerEvent>("runner", (e) => cb(e.payload));
}
