import { useEffect, useRef, type CSSProperties } from "react";
import { Cover } from "../components/Cover";
import { SYSTEMS, discLabel, type Game, type SystemId } from "../data/games";

/** Um item da fileira: um jogo, ou uma pasta de console (na estante "Todos"). */
export type ShelfItem = { kind: "game"; game: Game } | { kind: "folder"; system: SystemId; games: Game[] };

interface Props {
  items: ShelfItem[];
  selected: number;
  onSelect: (index: number) => void;
  onActivate: (item: ShelfItem) => void;
  onDetails: (game: Game) => void;
  onToggleFavorite: (game: Game) => void;
}

const TILE = 118;
const GAP = 14;

/**
 * Modo Console, no espírito da home do PS5: fileira de jogos no topo (o selecionado cresce e
 * mostra o nome ao lado), arte do jogo ocupando a tela e, embaixo, título grande e o botão Jogar.
 */
export function ConsoleView({ items, selected, onSelect, onActivate, onDetails, onToggleFavorite }: Props) {
  const item = items[selected];
  const rowRef = useRef<HTMLDivElement>(null);

  // A fileira desliza para a esquerda conforme anda, mantendo o selecionado sempre à vista.
  const offset = Math.max(0, selected - 1) * (TILE + GAP);

  useEffect(() => {
    rowRef.current?.querySelector<HTMLElement>(".tile.sel")?.focus({ preventScroll: true });
  }, [selected]);

  if (!item) return <section className="console empty">Nenhum jogo nesta estante ainda.</section>;

  return (
    <section className="console ps">
      <div className="row-wrap">
        <div className="row" ref={rowRef} style={{ transform: `translateX(${-offset}px)` }}>
          {items.map((it, i) => {
            const sel = i === selected;
            const key = it.kind === "game" ? `g${it.game.id}` : `f${it.system}`;
            const label = it.kind === "game" ? it.game.title : SYSTEMS[it.system].name;
            return (
              <div className={`tile-slot${sel ? " sel" : ""}`} key={key}>
                <button
                  className={`tile${sel ? " sel" : ""}${it.kind === "folder" ? " folder" : ""}`}
                  aria-label={label}
                  tabIndex={sel ? 0 : -1}
                  onClick={() => (sel ? onActivate(it) : onSelect(i))}
                >
                  {it.kind === "game" ? <Cover game={it.game} /> : <FolderArt system={it.system} games={it.games} />}
                </button>
                {sel && <span className="tile-name">{label}</span>}
              </div>
            );
          })}
        </div>
      </div>

      {item.kind === "game" ? (
        <GameHero game={item.game} onPlay={() => onActivate(item)} onDetails={onDetails} onToggleFavorite={onToggleFavorite} />
      ) : (
        <FolderHero system={item.system} games={item.games} onOpen={() => onActivate(item)} />
      )}
    </section>
  );
}

function GameHero({
  game,
  onPlay,
  onDetails,
  onToggleFavorite,
}: {
  game: Game;
  onPlay: () => void;
  onDetails: (g: Game) => void;
  onToggleFavorite: (g: Game) => void;
}) {
  const [earned, total] = game.trophies;
  return (
    <div className="hero" key={game.id}>
      <div className="hero-info">
        <div className="hero-sys">
          <i style={{ background: SYSTEMS[game.system].dot }} />
          {SYSTEMS[game.system].name}
          {game.demo && <span className="tag demo">Exemplo</span>}
        </div>
        <h1>{game.title}</h1>
        <div className="hero-meta">
          {[game.region, game.serial, discLabel(game), game.format, game.playtime ? `${game.playtime} jogadas` : "Nunca jogado", game.lastPlayed]
            .filter(Boolean)
            .join("  ·  ")}
        </div>
        {total > 0 && (
          <div className="trophy">
            <span aria-hidden="true">🏆</span>
            <div className="bar">
              <i style={{ width: `${(earned / total) * 100}%` }} />
            </div>
            <span>
              {earned} / {total}
            </span>
          </div>
        )}
        <div className="hero-actions">
          <button className="play-btn" onClick={onPlay}>
            <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
              <path d="M7 4.5v15l13-7.5z" />
            </svg>
            Jogar
          </button>
          <button
            className={`round${game.favorite ? " on" : ""}`}
            aria-label={game.favorite ? "Remover dos favoritos" : "Favoritar"}
            title={game.favorite ? "Remover dos favoritos" : "Favoritar"}
            onClick={() => onToggleFavorite(game)}
          >
            <svg viewBox="0 0 24 24" fill={game.favorite ? "currentColor" : "none"} stroke="currentColor" strokeWidth="1.8">
              <path d="M12 3.5l2.6 5.3 5.8.8-4.2 4.1 1 5.8L12 16.8l-5.2 2.7 1-5.8-4.2-4.1 5.8-.8z" />
            </svg>
          </button>
          <button className="round" aria-label="Detalhes" title="Detalhes" onClick={() => onDetails(game)}>
            <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
              <circle cx="5" cy="12" r="2" />
              <circle cx="12" cy="12" r="2" />
              <circle cx="19" cy="12" r="2" />
            </svg>
          </button>
        </div>
      </div>
      <div className="hero-art">
        <Cover game={game} />
      </div>
    </div>
  );
}

function FolderArt({ system, games }: { system: SystemId; games: Game[] }) {
  const four = games.slice(0, 4);
  return (
    <div className="folder-art" style={{ "--dot": SYSTEMS[system].dot } as CSSProperties}>
      <div className="mosaic">
        {four.map((g) => (
          <Cover key={g.id} game={g} />
        ))}
        {Array.from({ length: 4 - four.length }, (_, i) => (
          <span key={`v${i}`} className="empty" />
        ))}
      </div>
      <span className="folder-tag">{SYSTEMS[system].folder}</span>
    </div>
  );
}

function FolderHero({ system, games, onOpen }: { system: SystemId; games: Game[]; onOpen: () => void }) {
  const played = games.filter((g) => g.playtime).length;
  return (
    <div className="hero" key={`f${system}`}>
      <div className="hero-info">
        <div className="hero-sys">
          <i style={{ background: SYSTEMS[system].dot }} />
          Pasta
        </div>
        <h1>{SYSTEMS[system].name}</h1>
        <div className="hero-meta">
          {games.length} jogo{games.length === 1 ? "" : "s"}  ·  {played} já jogado{played === 1 ? "" : "s"}  ·  emulador {SYSTEMS[system].core}
        </div>
        <div className="hero-actions">
          <button className="play-btn" onClick={onOpen}>
            Abrir pasta
          </button>
        </div>
      </div>
      <div className="hero-art folder-big">
        <FolderArt system={system} games={games} />
      </div>
    </div>
  );
}
