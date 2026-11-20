import type { CSSProperties } from "react";
import { SYSTEMS, type Game } from "../data/games";

/** Capa gerada para os jogos de exemplo. Na Fase 2 vira a imagem real (box art). */
export function Cover({ game }: { game: Game }) {
  const style = { "--c1": game.c1, "--c2": game.c2 } as CSSProperties;
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
    </div>
  );
}
