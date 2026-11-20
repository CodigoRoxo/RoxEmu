import { SYSTEMS, type Game } from "../data/games";
import type { PadStyle } from "../hooks/usePadStyle";
import { FittedCover } from "./Cover";
import { PadGlyph } from "./PadGlyph";

export type MenuItemId = "play" | "fav" | "details" | "art" | "remove" | "uninstall";

export interface MenuItem {
  id: MenuItemId;
  label: string;
  hint?: string;
  danger?: boolean;
}

/** Opções do jogo (botão OPTIONS), como o menu "•••" do PS5. */
export function menuItems(game: Game): MenuItem[] {
  const items: MenuItem[] = [
    { id: "play", label: "Jogar" },
    { id: "fav", label: game.favorite ? "Remover dos favoritos" : "Favoritar" },
    { id: "details", label: "Detalhes" },
  ];
  if (!game.demo) {
    items.push(
      { id: "art", label: "Buscar capa e artes de novo" },
      { id: "remove", label: "Remover da estante", hint: "os arquivos continuam no disco" },
      { id: "uninstall", label: "Desinstalar", hint: "apaga o jogo do disco", danger: true },
    );
  }
  return items;
}

/** Popup de vidro com as opções do jogo. Navegável pelo controle (↑↓, ✕, ○). */
export function GameMenu({
  game,
  index,
  padStyle,
  onPick,
  onHover,
  onClose,
}: {
  game: Game;
  index: number;
  padStyle: PadStyle;
  onPick: (item: MenuItem) => void;
  onHover: (i: number) => void;
  onClose: () => void;
}) {
  const items = menuItems(game);
  return (
    <div className="glass-backdrop" onClick={onClose}>
      <section className="glass-menu" role="menu" aria-label={`Opções de ${game.title}`} onClick={(e) => e.stopPropagation()}>
        <header>
          <div className="gm-cover">
            <FittedCover game={game} />
          </div>
          <div className="gm-title">
            <small>{SYSTEMS[game.system].name}</small>
            <b>{game.title}</b>
          </div>
        </header>
        <ul>
          {items.map((it, i) => (
            <li key={it.id}>
              <button
                role="menuitem"
                className={`gm-item${i === index ? " on" : ""}${it.danger ? " danger" : ""}`}
                onMouseEnter={() => onHover(i)}
                onClick={() => onPick(it)}
              >
                <span>{it.label}</span>
                {it.hint && <small>{it.hint}</small>}
              </button>
            </li>
          ))}
        </ul>
        <footer>
          <span>
            <PadGlyph action="confirm" style={padStyle} />
            Escolher
          </span>
          <span>
            <PadGlyph action="back" style={padStyle} />
            Fechar
          </span>
        </footer>
      </section>
    </div>
  );
}

/**
 * Confirmação de desinstalar. Apagar não tem volta, então o foco começa em "Cancelar"
 * e o texto diz claramente o que acontece.
 */
export function UninstallConfirm({
  game,
  focus,
  padStyle,
  busy,
  onFocus,
  onConfirm,
  onCancel,
}: {
  game: Game;
  focus: 0 | 1;
  padStyle: PadStyle;
  busy: boolean;
  onFocus: (f: 0 | 1) => void;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="glass-backdrop strong" onClick={busy ? undefined : onCancel}>
      <section className="glass-menu confirm" role="alertdialog" aria-label="Desinstalar jogo" onClick={(e) => e.stopPropagation()}>
        <div className="gm-cover big">
          <FittedCover game={game} />
        </div>
        <h3>Desinstalar {game.title}?</h3>
        <p>
          Os arquivos do jogo (<b>{game.size}</b>) serão <b>apagados do disco para sempre</b>, sem passar
          pela lixeira. Isso não pode ser desfeito.
        </p>
        <p className="fine">
          Seus saves e memory cards continuam guardados no ROXemu. Arquivo: <code>{game.path ?? game.file}</code>
        </p>
        <div className="confirm-actions">
          <button className={`btn${focus === 0 ? " focus" : ""}`} onMouseEnter={() => onFocus(0)} onClick={onCancel} disabled={busy}>
            Cancelar
          </button>
          <button
            className={`btn danger-btn${focus === 1 ? " focus" : ""}`}
            onMouseEnter={() => onFocus(1)}
            onClick={onConfirm}
            disabled={busy}
          >
            {busy ? "Apagando…" : "Desinstalar"}
          </button>
        </div>
        <footer>
          <span>
            <PadGlyph action="confirm" style={padStyle} />
            Escolher
          </span>
          <span>
            <PadGlyph action="back" style={padStyle} />
            Cancelar
          </span>
        </footer>
      </section>
    </div>
  );
}
