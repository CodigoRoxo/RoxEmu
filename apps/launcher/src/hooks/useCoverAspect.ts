import { useEffect, useState } from "react";
import type { Game, SystemId } from "../data/games";

// Proporção (largura / altura) de cada capa, medida da imagem real e guardada em memória.
const cache = new Map<string, number>();
const pending = new Map<string, Promise<number>>();

/** Formato típico da caixa de cada console, usado até a imagem carregar. */
const TYPICAL: Record<SystemId, number> = {
  ps1: 1, // caixa de CD (jewel case): quase quadrada
  ps2: 0.71, // caixa de DVD: retrato
  snes: 1.38, // caixa de cartucho (EUA/Europa): paisagem
  gba: 1,
};

function measure(url: string): Promise<number> {
  const known = pending.get(url);
  if (known) return known;
  const p = new Promise<number>((resolve) => {
    const img = new Image();
    img.onload = () => {
      const ar = img.naturalWidth && img.naturalHeight ? img.naturalWidth / img.naturalHeight : 1;
      // Limita extremos (imagens estranhas não podem quebrar a fileira).
      const clamped = Math.min(1.8, Math.max(0.55, ar));
      cache.set(url, clamped);
      resolve(clamped);
    };
    img.onerror = () => resolve(1);
    img.src = url;
  });
  pending.set(url, p);
  return p;
}

/** Proporção da capa do jogo: medida da imagem real; capas geradas são quadradas. */
export function useCoverAspect(game: Game | undefined): number {
  const url = game?.coverUrl;
  const fallback = !game ? 1 : url ? TYPICAL[game.system] : 1;
  const [ar, setAr] = useState(() => (url && cache.get(url)) || fallback);

  useEffect(() => {
    if (!url) {
      setAr(1);
      return;
    }
    const known = cache.get(url);
    if (known) {
      setAr(known);
      return;
    }
    setAr(fallback);
    let alive = true;
    measure(url).then((v) => alive && setAr(v));
    return () => {
      alive = false;
    };
  }, [url, fallback]);

  return ar;
}

/** Tamanho em pixels para caber numa altura fixa (fileira, capa grande). */
export function sizeForHeight(ar: number, height: number) {
  return { width: Math.round(height * ar), height };
}
