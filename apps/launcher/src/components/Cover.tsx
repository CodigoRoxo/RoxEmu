import { useState, type CSSProperties } from "react";
import { SYSTEMS, type Game } from "../data/games";
import { useCoverAspect } from "../hooks/useCoverAspect";

/**
 * Capa encaixada num espaço qualquer sem cortar nem desfocar: retrato ocupa a altura,
 * paisagem ocupa a largura, sempre no formato real da imagem (alinhada embaixo, como numa estante).
 */
export function FittedCover({ game, align = "center" }: { game: Game; align?: "center" | "start" }) {
  const ar = useCoverAspect(game);
  const frame: CSSProperties = ar <= 1 ? { height: "100%", aspectRatio: ar } : { width: "100%", aspectRatio: ar };
  return (
    <div className="cover-fit" style={{ justifyContent: align === "start" ? "flex-start" : "center" }}>
      <div className="cover-frame" style={frame}>
        <Cover game={game} />
      </div>
    </div>
  );
}

/** Indicador de carregamento estilo iOS (12 tracinhos acendendo em roda). */
export function IosSpinner({ label }: { label?: string }) {
  return (
    <div className="ios-spin" role="status" aria-label={label ?? "Carregando"}>
      <div className="s">
        {Array.from({ length: 12 }, (_, i) => (
          <i
            key={i}
            style={{ transform: `rotate(${i * 30}deg)`, animationDelay: `${-((12 - i) / 12).toFixed(3)}s` }}
          />
        ))}
      </div>
    </div>
  );
}

/** Capa do jogo: a imagem real quando existe; senão, uma capa gerada com as cores do ROXemu. */
export function Cover({ game }: { game: Game }) {
  const [broken, setBroken] = useState<string | null>(null);
  const style = { "--c1": game.c1, "--c2": game.c2 } as CSSProperties;
  const spinner = game.coverLoading ? <IosSpinner label={`Buscando a capa de ${game.title}`} /> : null;

  if (game.coverUrl && broken !== game.coverUrl) {
    return (
      <div className="cover real" style={style}>
        {/* Quem usa a capa dá a ela o formato da imagem (useCoverAspect): nada é cortado nem desfocado. */}
        <img
          className="main"
          src={game.coverUrl}
          alt={`Capa de ${game.title}`}
          draggable={false}
          onError={() => setBroken(game.coverUrl!)}
        />
        {spinner}
      </div>
    );
  }

  return (
    <div className="cover" style={style}>
      <div className="art">
        <div className={`m m-${game.motif}`} />
      </div>
      <div className="spine">
        <span>ROX</span>
        <span>{SYSTEMS[game.system].folder}</span>
      </div>
      {game.region && <span className="rg">{game.region}</span>}
      <div className="ct">{game.title}</div>
      {spinner}
    </div>
  );
}
