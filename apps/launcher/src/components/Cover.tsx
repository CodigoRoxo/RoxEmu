import { useState, type CSSProperties } from "react";
import { SYSTEMS, type Game } from "../data/games";

/** Capa do jogo: a imagem real quando existe; senão, uma capa gerada com as cores do ROXemu. */
export function Cover({ game }: { game: Game }) {
  const [broken, setBroken] = useState<string | null>(null);
  const style = { "--c1": game.c1, "--c2": game.c2 } as CSSProperties;

  if (game.coverUrl && broken !== game.coverUrl) {
    return (
      <div className="cover real" style={style}>
        <img src={game.coverUrl} alt={`Capa de ${game.title}`} draggable={false} onError={() => setBroken(game.coverUrl!)} />
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
    </div>
  );
}
