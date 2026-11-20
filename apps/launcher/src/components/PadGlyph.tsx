import type { PadStyle } from "../hooks/usePadStyle";

/** Ações da interface, e o botão de cada uma (igual ao PS5: ✕ confirma, ○ volta). */
export type PadAction = "confirm" | "back" | "details" | "fav" | "l1" | "r1" | "options" | "create";

const XBOX: Record<PadAction, string> = {
  confirm: "A",
  back: "B",
  details: "Y",
  fav: "X",
  l1: "LB",
  r1: "RB",
  options: "≡",
  create: "⧉",
};

const PS_TEXT: Partial<Record<PadAction, string>> = {
  l1: "L1",
  r1: "R1",
  options: "OPTIONS",
  create: "CREATE",
};

/** Símbolo do botão do controle, no estilo do controle conectado. */
export function PadGlyph({ action, style }: { action: PadAction; style: PadStyle }) {
  if (style === "xbox") {
    const text = XBOX[action];
    const face = ["confirm", "back", "details", "fav"].includes(action);
    return <b className={`pad-glyph xb ${face ? `face ${action}` : "pill"}`}>{text}</b>;
  }
  const text = PS_TEXT[action];
  if (text) return <b className={`pad-glyph ps pill${text.length > 2 ? " wide" : ""}`}>{text}</b>;
  return (
    <b className={`pad-glyph ps face ${action}`} aria-label={action}>
      <svg viewBox="0 0 20 20" aria-hidden="true">
        {action === "confirm" && <path d="M6.2 6.2l7.6 7.6M13.8 6.2l-7.6 7.6" />}
        {action === "back" && <circle cx="10" cy="10" r="4.6" />}
        {action === "fav" && <rect x="5.6" y="5.6" width="8.8" height="8.8" rx="0.6" />}
        {action === "details" && <path d="M10 5.2l5 8.6H5z" />}
      </svg>
    </b>
  );
}
