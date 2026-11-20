import { useEffect } from "react";

/**
 * Animação de "ligar o console": um feixe de luz cruza a tela, o logo ROX acende com um
 * brilho roxo e a frase aparece. Qualquer tecla, botão ou clique pula.
 */
export function BootScreen({ onDone }: { onDone: () => void }) {
  useEffect(() => {
    const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const t = setTimeout(onDone, still ? 600 : 3200);
    return () => clearTimeout(t);
  }, [onDone]);

  return (
    <div className="boot" onClick={onDone} role="presentation">
      <div className="boot-beam" />
      <div className="boot-ring" />
      <div className="boot-logo">
        ROX<b>emu</b>
      </div>
      <div className="boot-tag">seus jogos, num lugar só</div>
    </div>
  );
}
