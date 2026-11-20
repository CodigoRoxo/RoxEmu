import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface AppInfo {
  version: string;
  data: { root: string; bios: string; portable: boolean };
  systems: {
    id: string;
    name: string;
    extensions: string[];
    default_core: string;
    requires_bios: boolean;
    has_bios: boolean;
  }[];
}

/** Jogo como o backend guarda (library.json). */
export interface LibraryGame {
  id: number;
  system: "ps1" | "ps2" | "ps3" | "snes" | "gba";
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
  hero: string | null;
  logo: string | null;
  icon: string | null;
  art_from_franchise: boolean;
}

export interface SettingsView {
  has_steamgriddb_key: boolean;
  steamgriddb_key_hint: string | null;
}

export async function getSettings(): Promise<SettingsView | null> {
  return inApp ? invoke<SettingsView>("get_settings") : null;
}
/** Testa e salva a chave do SteamGridDB (vazia = remove). Lança erro se a chave não funcionar. */
export const setSteamgriddbKey = (key: string) => invoke<void>("set_steamgriddb_key", { key });
export const refreshArt = (id: number) => invoke<void>("refresh_art", { id });
export const openSteamgriddbPage = () => invoke<void>("open_steamgriddb_page");

export function onArtError(cb: (message: string) => void): Promise<UnlistenFn> {
  if (!inApp) return Promise.resolve(() => {});
  return listen<string>("art-error", (e) => cb(e.payload));
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
/** `pad`: controle conectado ("dualsense" | "ds4" | "xbox"), para o emulador já vir mapeado. */
export const launchGame = (id: number, pad: string | null) => invoke<void>("launch_game", { id, pad });

/** Jogo de PS3 em pasta: abre o seletor de pastas. `null` se cancelar. */
export async function addGameFolder(): Promise<LibraryGame | null> {
  if (!inApp) throw new Error("Adicionar jogos só funciona dentro do app ROXemu.");
  return invoke<LibraryGame | null>("add_game_folder");
}

export interface Ps3Status {
  emulator_found: boolean;
  firmware: string | null;
}
export async function ps3Status(): Promise<Ps3Status | null> {
  return inApp ? invoke<Ps3Status>("ps3_status") : null;
}
export const openPs3FirmwarePage = () => invoke<void>("open_ps3_firmware_page");
/** Escolhe o PS3UPDAT.PUP e abre a instalação no RPCS3. `false` se cancelar. */
export const installPs3Firmware = () => invoke<boolean>("install_ps3_firmware");
/** Avisado quando a janela de instalação do firmware fecha (versão instalada, ou null). */
export function onPs3Firmware(cb: (version: string | null) => void): Promise<UnlistenFn> {
  if (!inApp) return Promise.resolve(() => {});
  return listen<string | null>("ps3-firmware", (e) => cb(e.payload));
}
export const openBiosFolder = () => invoke<void>("open_bios_folder");

export const refreshCover = (id: number) => invoke<void>("refresh_cover", { id });
/** Escolher imagem própria como capa. `false` se cancelar. */
export const chooseCover = (id: number) => invoke<boolean>("choose_cover", { id });

/** Avisado quando algo muda na biblioteca em segundo plano (ex.: capa baixada). Recebe o id do jogo. */
export function onLibraryChanged(cb: (gameId: number) => void): Promise<UnlistenFn> {
  if (!inApp) return Promise.resolve(() => {});
  return listen<number>("library-changed", (e) => cb(e.payload));
}

/** Avisado quando começa a busca da capa de um jogo. */
export function onCoverLoading(cb: (gameId: number) => void): Promise<UnlistenFn> {
  if (!inApp) return Promise.resolve(() => {});
  return listen<number>("cover-loading", (e) => cb(e.payload));
}

/** Avisado quando a busca de capa de um jogo não encontrou nada (ou falhou). */
export function onCoverMissing(
  cb: (e: { id: number; title: string; error: string | null }) => void,
): Promise<UnlistenFn> {
  if (!inApp) return Promise.resolve(() => {});
  return listen<{ id: number; title: string; error: string | null }>("cover-missing", (e) => cb(e.payload));
}

/** URL que a interface usa para mostrar um arquivo de capa do disco. */
export function fileUrl(path: string): string {
  return convertFileSrc(path);
}

export function onRunnerEvent(cb: (e: RunnerEvent) => void): Promise<UnlistenFn> {
  if (!inApp) return Promise.resolve(() => {});
  return listen<RunnerEvent>("runner", (e) => cb(e.payload));
}
