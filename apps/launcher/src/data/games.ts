// Jogos da estante: os do usuário (library.json) e os exemplos fictícios, mostrados só
// enquanto a biblioteca está vazia.

import { fileUrl, type LibraryGame } from "../lib/backend";

export type SystemId = "ps1" | "snes" | "gba";
export type Motif = "sun" | "moon" | "stripes" | "ring" | "star" | "grid";

export interface Game {
  id: number;
  /** Jogo de exemplo (não existe arquivo de verdade). */
  demo: boolean;
  title: string;
  system: SystemId;
  region: string | null;
  serial: string | null;
  /** 0 = cartucho */
  discs: number;
  format: string;
  size: string;
  playtime: string | null;
  lastPlayed: string | null;
  recent: boolean;
  trophies: [earned: number, total: number];
  favorite: boolean;
  file: string;
  /** Caminho completo do arquivo (só jogos reais). */
  path?: string;
  /** Capa de verdade (URL para o <img>), quando já foi baixada. */
  coverUrl?: string;
  /** Cores da capa gerada (substituída pela capa real na Fase 2). */
  c1: string;
  c2: string;
  motif: Motif;
}

export const SYSTEMS: Record<SystemId, { name: string; core: string; dot: string; folder: string }> = {
  ps1: { name: "PlayStation", core: "SwanStation", dot: "#a78bfa", folder: "PS1" },
  snes: { name: "Super Nintendo", core: "Snes9x", dot: "#f472b6", folder: "SNES" },
  gba: { name: "Game Boy Advance", core: "mGBA", dot: "#60a5fa", folder: "GBA" },
};

const RAW_SAMPLES: Omit<Game, "demo">[] = [
  { id: 1, title: "Neon Drift", system: "ps1", region: "NTSC-U", serial: "SLUS-01482", discs: 1, format: "BIN/CUE · 3 faixas", size: "612 MB", playtime: "12h 40min", lastPlayed: "Ontem", recent: true, trophies: [8, 24], favorite: true, file: "Neon Drift (USA).cue", c1: "#3b1a7a", c2: "#ff4fa3", motif: "sun" },
  { id: 2, title: "Lenda de Aurora", system: "ps1", region: "NTSC-J", serial: "SLPS-02215", discs: 3, format: "M3U · 3 discos (CHD)", size: "1,4 GB", playtime: "58h 12min", lastPlayed: "Hoje", recent: true, trophies: [22, 40], favorite: true, file: "Lenda de Aurora (Japan).m3u", c1: "#2e1065", c2: "#f0abfc", motif: "star" },
  { id: 3, title: "Castelo Sombrio", system: "ps1", region: "NTSC-U", serial: "SLUS-00733", discs: 1, format: "CHD", size: "431 MB", playtime: "31h 05min", lastPlayed: "Há 3 dias", recent: true, trophies: [19, 30], favorite: false, file: "Castelo Sombrio (USA).chd", c1: "#1e0a2e", c2: "#e11d74", motif: "moon" },
  { id: 4, title: "Abismo", system: "ps1", region: "NTSC-U", serial: "SLUS-01390", discs: 2, format: "M3U · 2 discos (BIN/CUE)", size: "1,2 GB", playtime: null, lastPlayed: null, recent: false, trophies: [0, 20], favorite: false, file: "Abismo (USA).m3u", c1: "#06141f", c2: "#22d3ee", motif: "ring" },
  { id: 5, title: "Rally 98", system: "ps1", region: "PAL", serial: "SCES-01120", discs: 1, format: "ISO", size: "498 MB", playtime: "4h 10min", lastPlayed: "Semana passada", recent: false, trophies: [3, 15], favorite: false, file: "Rally 98 (Europe).iso", c1: "#0c3b4a", c2: "#f59e0b", motif: "stripes" },
  { id: 6, title: "Furtivo", system: "ps1", region: "NTSC-U", serial: "SLUS-01177", discs: 2, format: "PBP", size: "980 MB", playtime: "26h 30min", lastPlayed: "Mês passado", recent: false, trophies: [14, 32], favorite: false, file: "Furtivo (USA).pbp", c1: "#14101c", c2: "#ef4444", motif: "moon" },
  { id: 7, title: "Mecha Rox", system: "ps1", region: "NTSC-U", serial: "SLUS-00988", discs: 1, format: "BIN/CUE · 1 faixa", size: "540 MB", playtime: "7h 02min", lastPlayed: "Há 2 meses", recent: false, trophies: [5, 18], favorite: false, file: "Mecha Rox (USA).cue", c1: "#120b24", c2: "#a855f7", motif: "grid" },
  { id: 8, title: "Céu de Papel", system: "ps1", region: "PAL", serial: "SCES-02041", discs: 1, format: "CHD", size: "302 MB", playtime: "15h 48min", lastPlayed: "Há 3 semanas", recent: false, trophies: [11, 25], favorite: true, file: "Ceu de Papel (Europe).chd", c1: "#5b0f2a", c2: "#fb923c", motif: "sun" },
  { id: 9, title: "Pixel Knights", system: "snes", region: "NTSC-U", serial: "SNS-PKNE-USA", discs: 0, format: "SFC (zip)", size: "2 MB", playtime: "9h 20min", lastPlayed: "Há 2 semanas", recent: false, trophies: [6, 20], favorite: false, file: "Pixel Knights (USA).zip", c1: "#4a1a08", c2: "#fde047", motif: "grid" },
  { id: 10, title: "Ondas 2000", system: "snes", region: "PAL", serial: "SNSP-OW2P-EUR", discs: 0, format: "SFC", size: "3 MB", playtime: "1h 15min", lastPlayed: "Há 4 meses", recent: false, trophies: [1, 12], favorite: false, file: "Ondas 2000 (Europe).sfc", c1: "#053b2f", c2: "#a3e635", motif: "ring" },
  { id: 11, title: "Turbo Mini", system: "gba", region: "NTSC-U", serial: "AGB-BTME-USA", discs: 0, format: "GBA (zip)", size: "8 MB", playtime: "2h 05min", lastPlayed: "Mês passado", recent: false, trophies: [2, 10], favorite: false, file: "Turbo Mini (USA).zip", c1: "#11307a", c2: "#f472b6", motif: "stripes" },
  { id: 12, title: "Bolha Bros", system: "gba", region: "NTSC-J", serial: "AGB-BBRJ-JPN", discs: 0, format: "GBA", size: "16 MB", playtime: null, lastPlayed: null, recent: false, trophies: [0, 8], favorite: false, file: "Bolha Bros (Japan).gba", c1: "#6b0f3e", c2: "#fcd34d", motif: "star" },
];

export const SAMPLE_GAMES: Game[] = RAW_SAMPLES.map((g) => ({ ...g, id: -g.id, demo: true }));

// ---------- jogos reais ----------

const PALETTE: [string, string][] = [
  ["#3b1a7a", "#ff4fa3"],
  ["#2e1065", "#f0abfc"],
  ["#1e0a2e", "#e11d74"],
  ["#06141f", "#22d3ee"],
  ["#0c3b4a", "#f59e0b"],
  ["#120b24", "#a855f7"],
  ["#5b0f2a", "#fb923c"],
  ["#11307a", "#f472b6"],
];
const MOTIFS: Motif[] = ["sun", "star", "moon", "ring", "stripes", "grid"];

function hash(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619);
  return h >>> 0;
}

export function formatPlaytime(secs: number): string | null {
  if (secs < 60) return null;
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return h > 0 ? `${h}h ${String(m).padStart(2, "0")}min` : `${m}min`;
}

export function formatLastPlayed(unix: number | null): string | null {
  if (!unix) return null;
  const days = Math.floor((Date.now() / 1000 - unix) / 86400);
  if (days <= 0) return "Hoje";
  if (days === 1) return "Ontem";
  if (days < 7) return `Há ${days} dias`;
  if (days < 30) return `Há ${Math.floor(days / 7)} semana${days < 14 ? "" : "s"}`;
  return `Há ${Math.floor(days / 30)} ${days < 60 ? "mês" : "meses"}`;
}

function formatSize(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1).replace(".", ",")} GB`;
  return `${Math.max(1, Math.round(bytes / 1024 ** 2))} MB`;
}

/** Converte o jogo do backend para o formato da estante (com capa gerada até a Fase 2). */
export function fromLibrary(g: LibraryGame): Game {
  const h = hash(g.title);
  const [c1, c2] = PALETTE[h % PALETTE.length];
  const file = g.path.split(/[\\/]/).pop() ?? g.path;
  return {
    id: g.id,
    demo: false,
    title: g.title,
    system: g.system,
    region: g.region,
    serial: g.serial,
    discs: g.discs,
    format: g.format,
    size: formatSize(g.size_bytes),
    playtime: formatPlaytime(g.playtime_secs),
    lastPlayed: formatLastPlayed(g.last_played),
    recent: !!g.last_played && Date.now() / 1000 - g.last_played < 7 * 86400,
    trophies: [0, 0],
    favorite: g.favorite,
    file,
    path: g.path,
    coverUrl: g.cover ? fileUrl(g.cover) : undefined,
    c1,
    c2,
    motif: MOTIFS[(h >>> 8) % MOTIFS.length],
  };
}

export interface Category {
  label: string;
  filter: (g: Game) => boolean;
}

export const CATEGORIES: Category[] = [
  { label: "Todos", filter: () => true },
  { label: "Jogados recentemente", filter: (g) => g.recent },
  { label: "Favoritos", filter: (g) => g.favorite },
  { label: "PlayStation", filter: (g) => g.system === "ps1" },
  { label: "Super Nintendo", filter: (g) => g.system === "snes" },
  { label: "Game Boy Advance", filter: (g) => g.system === "gba" },
  { label: "Multi-disco", filter: (g) => g.discs > 1 },
  { label: "Japão", filter: (g) => g.region === "NTSC-J" },
];

export function discLabel(g: Game): string {
  if (g.discs === 0) return "Cartucho";
  return g.discs === 1 ? "1 disco" : `${g.discs} discos`;
}
