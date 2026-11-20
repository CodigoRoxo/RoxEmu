import { useCallback, useEffect, useState } from "react";

/** Quais símbolos mostrar na tela. */
export type PadStyle = "ps" | "xbox";
/** Preferência do usuário: seguir o controle conectado ou fixar um estilo. */
export type PadPref = "auto" | PadStyle;

const KEY = "rox.padStyle";

function loadPref(): PadPref {
  try {
    const v = localStorage.getItem(KEY);
    if (v === "auto" || v === "ps" || v === "xbox") return v;
  } catch {
    // sem localStorage
  }
  return "auto";
}

/** Controle da Sony? (DualShock 3/4, DualSense; 054c é o código de fabricante da Sony) */
export function isPlayStationPad(id: string): boolean {
  return /054c|dualsense|dualshock|playstation|wireless controller/i.test(id);
}

function detect(): PadStyle | null {
  const pads = navigator.getGamepads?.() ?? [];
  const pad = pads.find((p) => p !== null);
  if (!pad) return null;
  return isPlayStationPad(pad.id) ? "ps" : "xbox";
}

/**
 * Modelo do controle conectado, para o emulador (RPCS3) já vir mapeado:
 * DualSense (PS5), DualShock 4 (PS4) ou Xbox. `null` sem controle.
 */
export function currentPadKind(): "dualsense" | "ds4" | "xbox" | null {
  const pad = (navigator.getGamepads?.() ?? []).find((p) => p !== null);
  if (!pad) return null;
  const id = pad.id.toLowerCase();
  if (/dualsense|0ce6|0df2/.test(id)) return "dualsense";
  if (isPlayStationPad(id)) return "ds4";
  return "xbox";
}

/**
 * Estilo dos símbolos de botão. No automático segue o controle conectado; sem controle, usa
 * PlayStation (o padrão do ROXemu).
 */
export function usePadStyle() {
  const [pref, setPrefState] = useState<PadPref>(loadPref);
  const [detected, setDetected] = useState<PadStyle | null>(() => detect());

  useEffect(() => {
    const update = () => setDetected(detect());
    window.addEventListener("gamepadconnected", update);
    window.addEventListener("gamepaddisconnected", update);
    // Alguns navegadores só listam o controle depois do primeiro botão apertado.
    const t = setInterval(update, 2000);
    return () => {
      window.removeEventListener("gamepadconnected", update);
      window.removeEventListener("gamepaddisconnected", update);
      clearInterval(t);
    };
  }, []);

  const setPref = useCallback((p: PadPref) => {
    setPrefState(p);
    try {
      localStorage.setItem(KEY, p);
    } catch {
      // ignora
    }
  }, []);

  const style: PadStyle = pref === "auto" ? (detected ?? "ps") : pref;
  return { style, pref, setPref, detected };
}
