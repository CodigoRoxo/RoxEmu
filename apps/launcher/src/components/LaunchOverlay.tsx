import { SYSTEMS, type Game } from "../data/games";
import { Cover } from "./Cover";

/** Tela de "iniciando". Na Fase 3 ela some quando o runner avisa que o jogo abriu. */
export function LaunchOverlay({ game, resume, onClose }: { game: Game; resume: boolean; onClose: () => void }) {
  return (
    <div className="launch" onClick={onClose} role="dialog" aria-label={`Iniciando ${game.title}`}>
      <div className="box">
        <Cover game={game} />
        <h3>
          {resume ? "Carregando save de" : "Iniciando"} {game.title}
        </h3>
        <div className="bar">
          <i />
        </div>
        <p>
          {SYSTEMS[game.system].core} · {game.format}. O runner que abre o jogo chega na Fase 1. Aperte B ou Esc para
          voltar.
        </p>
      </div>
    </div>
  );
}
