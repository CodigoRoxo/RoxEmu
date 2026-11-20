import { useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import { Cover } from "../components/Cover";
import { SYSTEMS, discLabel, type Game } from "../data/games";

interface Props {
  games: Game[];
  selected: number;
  onSelect: (index: number) => void;
  onLaunch: (game: Game, resume?: boolean) => void;
  onDetails: (game: Game) => void;
  onToggleFavorite: (game: Game) => void;
}

/** Modo Console: capa grande no centro, as próximas menores à direita. */
export function ConsoleView({ games, selected, onSelect, onLaunch, onDetails, onToggleFavorite }: Props) {
  const stage = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });

  useLayoutEffect(() => {
    const el = stage.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setSize({ w: el.clientWidth, h: el.clientHeight }));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const game = games[selected];
  const big = Math.max(140, Math.min(size.h * 0.9, size.w * 0.46, 400));
  const small = big * 0.66;
  const gap = big * 0.08;
  const cx = size.w / 2;

  const slotStyle = (i: number): CSSProperties => {
    const d = i - selected;
    let scale = small / big;
    let x: number;
    let opacity = 1;
    if (d === 0) {
      scale = 1;
      x = cx;
    } else if (d > 0) {
      x = cx + big / 2 + gap + (d - 1) * (small + gap * 0.6) + small / 2;
    } else {
      x = cx - big / 2 - gap - (-d - 1) * (small + gap * 0.6) - small / 2;
      opacity = d === -1 ? 0.45 : 0;
    }
    if (x - small / 2 > size.w || x + small / 2 < 0) opacity = 0;
    return {
      width: big,
      height: big,
      opacity,
      zIndex: 100 - Math.abs(d),
      transform: `translate(${x - big / 2}px, ${(size.h - big) / 2}px) scale(${scale})`,
    };
  };

  if (!game) return <section className="console empty">Nenhum jogo nesta categoria.</section>;
  const [earned, total] = game.trophies;

  return (
    <section className="console">
      <div className="stage" ref={stage}>
        {size.w > 0 &&
          games.map((g, i) => (
            <div
              key={g.id}
              className={`slot${i === selected ? " sel" : ""}`}
              style={slotStyle(i)}
              role="button"
              tabIndex={i === selected ? 0 : -1}
              aria-label={g.title}
              onClick={() => (i === selected ? onLaunch(g) : onSelect(i))}
            >
              <Cover game={g} />
            </div>
          ))}
      </div>

      <div className="info">
        <h1>
          <button
            className={`fav${game.favorite ? " on" : ""}`}
            aria-label={game.favorite ? "Remover dos favoritos" : "Favoritar"}
            onClick={() => onToggleFavorite(game)}
          >
            <svg viewBox="0 0 24 24" fill={game.favorite ? "currentColor" : "none"} stroke="currentColor" strokeWidth="1.8">
              <path d="M12 3.5l2.6 5.3 5.8.8-4.2 4.1 1 5.8L12 16.8l-5.2 2.7 1-5.8-4.2-4.1 5.8-.8z" />
            </svg>
          </button>
          <span>{game.title}</span>
        </h1>
        <div className="tags">
          <span className="tag">{SYSTEMS[game.system].name}</span>
          <span className="tag">{game.region}</span>
          <span className="tag mono">{game.serial}</span>
          <span className="tag">{discLabel(game)}</span>
          <span className="tag fmt">{game.format}</span>
          <span className="tag">⏱ {game.playtime ?? "Nunca jogado"}</span>
          {game.lastPlayed && <span className="tag">{game.lastPlayed}</span>}
        </div>
        <div className="trophy">
          <span aria-hidden="true">🏆</span>
          <div className="bar">
            <i style={{ width: `${total ? (earned / total) * 100 : 0}%` }} />
          </div>
          <span>
            {earned} / {total}
          </span>
        </div>
        <div className="actions">
          <button className="btn play" onClick={() => onLaunch(game)}>
            <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
              <path d="M7 4.5v15l13-7.5z" />
            </svg>
            Jogar
          </button>
          {game.playtime && (
            <button className="btn" onClick={() => onLaunch(game, true)}>
              Continuar do save
            </button>
          )}
          <button className="btn" onClick={() => onDetails(game)}>
            Detalhes
          </button>
        </div>
      </div>
    </section>
  );
}
