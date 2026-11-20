import { SYSTEMS, type Game } from "../data/games";
import type { PadStyle } from "../hooks/usePadStyle";
import { FittedCover } from "./Cover";
import { PadGlyph } from "./PadGlyph";

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
  onOpenSettings: () => void;
  onAdd: () => void;
  onStop: () => void;
  padStyle: PadStyle;
}

/** Tela sobre a estante enquanto um jogo abre, roda ou dá erro. */
export function LaunchOverlay({ session, onClose, onOpenBios, onOpenSettings, onAdd, onStop, padStyle }: Props) {
  const { game, phase, message } = session;
  const core = SYSTEMS[game.system].core;
  const biosMissing = phase === "error" && !!message?.includes("BIOS");
  const firmwareMissing = phase === "error" && !!message?.includes("firmware");
  const closable = phase !== "starting" && phase !== "playing";

  return (
    <div className={`launch ${phase}`} role="dialog" aria-label={game.title} onClick={closable ? onClose : undefined}>
      <div className="box" onClick={(e) => e.stopPropagation()}>
        <div className="launch-cover">
          <FittedCover game={game} />
        </div>

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
              {game.system === "ps3"
                ? `${game.title} está aberto no RPCS3. Nas primeiras vezes o emulador traduz o código do jogo enquanto ele roda: telas de carregamento podem ficar paradas por alguns minutos.`
                : `${game.title} está aberto na janela do jogo.`}
            </p>
            <span className="combo-hint">
              Para voltar ao console: segure <PadGlyph action="create" style={padStyle} /> +{" "}
              <PadGlyph action="options" style={padStyle} /> ou aperte o botão PS
            </span>
            <button className="btn play close-game" onClick={onStop}>
              Fechar jogo
            </button>
          </>
        )}

        {phase === "error" && (
          <>
            <h3>
              {biosMissing
                ? `Falta a BIOS do ${SYSTEMS[game.system].name}`
                : firmwareMissing
                  ? "Falta o firmware do PlayStation 3"
                  : "Não deu para abrir o jogo"}
            </h3>
            <p className="err">{message}</p>
            <div className="actions">
              {biosMissing && (
                <button className="btn play" onClick={onOpenBios}>
                  Abrir pasta da BIOS
                </button>
              )}
              {firmwareMissing && (
                <button className="btn play" onClick={onOpenSettings}>
                  Instalar firmware
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
