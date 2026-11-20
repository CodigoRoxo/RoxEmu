import { SYSTEMS, type Game } from "../data/games";
import { Cover } from "./Cover";

export type LaunchPhase = "starting" | "playing" | "error" | "demo";

export interface LaunchSession {
  game: Game;
  phase: LaunchPhase;
  message?: string;
}

interface Props {
  session: LaunchSession;
  onClose: () => void;
  onOpenBios: () => void;
  onAdd: () => void;
}

/** Tela sobre a estante enquanto um jogo abre, roda ou dá erro. */
export function LaunchOverlay({ session, onClose, onOpenBios, onAdd }: Props) {
  const { game, phase, message } = session;
  const core = SYSTEMS[game.system].core;
  const biosMissing = phase === "error" && !!message?.includes("BIOS");
  const closable = phase !== "starting" && phase !== "playing";

  return (
    <div className={`launch ${phase}`} role="dialog" aria-label={game.title} onClick={closable ? onClose : undefined}>
      <div className="box" onClick={(e) => e.stopPropagation()}>
        <Cover game={game} />

        {phase === "starting" && (
          <>
            <h3>Iniciando {game.title}</h3>
            <div className="bar">
              <i />
            </div>
            <p>
              {core} · {game.format}
            </p>
          </>
        )}

        {phase === "playing" && (
          <>
            <h3>Jogando agora</h3>
            <p>
              {game.title} está aberto na janela do jogo. Para voltar para a estante, aperte Esc ou o botão Guide do
              controle.
            </p>
          </>
        )}

        {phase === "error" && (
          <>
            <h3>{biosMissing ? "Falta a BIOS do PlayStation" : "Não deu para abrir o jogo"}</h3>
            <p className="err">{message}</p>
            <div className="actions">
              {biosMissing && (
                <button className="btn play" onClick={onOpenBios}>
                  Abrir pasta da BIOS
                </button>
              )}
              <button className="btn" onClick={onClose}>
                Voltar
              </button>
            </div>
          </>
        )}

        {phase === "demo" && (
          <>
            <h3>{game.title} é só um exemplo</h3>
            <p>
              Os jogos da estante são exemplos até você adicionar os seus. Escolha um arquivo de PlayStation (CUE,
              CHD, ISO, PBP ou M3U) e ele aparece aqui, pronto para jogar.
            </p>
            <div className="actions">
              <button className="btn play" onClick={onAdd}>
                + Adicionar jogo
              </button>
              <button className="btn" onClick={onClose}>
                Voltar
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
