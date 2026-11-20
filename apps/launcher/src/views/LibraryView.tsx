import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { FittedCover } from "../components/Cover";
import { SYSTEMS, type Game, type SystemId } from "../data/games";

interface Props {
  games: Game[];
  selectedId: number | null;
  onSelect: (game: Game) => void;
  onLaunch: (game: Game) => void;
  onAdd: () => void;
  onAddFolder: () => void;
  onRemove: (game: Game) => void;
  onCover: (game: Game, kind: "web" | "file") => void;
}

/** Agrupa por console, na ordem em que aparecem; é a ordem da grade (e da navegação pelo controle). */
export function groupBySystem(games: Game[]): [SystemId, Game[]][] {
  const by = new Map<SystemId, Game[]>();
  for (const g of games) by.set(g.system, [...(by.get(g.system) ?? []), g]);
  return [...by.entries()];
}

/** Jogos na ordem exata em que a grade mostra. */
export function gridOrder(games: Game[]): Game[] {
  return groupBySystem(games).flatMap(([, list]) => list);
}

/** Modo Biblioteca: grade agrupada por console e painel de detalhes. */
export function LibraryView({ games, selectedId, onSelect, onLaunch, onAdd, onAddFolder, onRemove, onCover }: Props) {
  const [query, setQuery] = useState("");

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return games.filter(
      (g) => !q || g.title.toLowerCase().includes(q) || (g.serial ?? "").toLowerCase().includes(q),
    );
  }, [games, query]);

  const groups = useMemo(() => groupBySystem(visible), [visible]);

  const selected = visible.find((g) => g.id === selectedId) ?? gridOrder(visible)[0] ?? null;

  // Navegando pelo controle: mantém o card selecionado à vista.
  useEffect(() => {
    document.querySelector(".library .card.sel")?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [selected?.id]);

  return (
    <section className="library">
      <aside className="side">
        <button className="nav on">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true">
            <rect x="3" y="4" width="7" height="16" rx="1.5" />
            <rect x="14" y="4" width="7" height="16" rx="1.5" />
          </svg>
          Biblioteca<span className="n">{games.length}</span>
        </button>
        <button className="nav">Configurações</button>
        <button className="nav">Troféus</button>
        <button className="nav">Logs</button>
        <h4>Consoles</h4>
        {Object.values(SYSTEMS).map((s) => (
          <div className="sys" key={s.name}>
            <i style={{ background: s.dot }} />
            {s.name}
          </div>
        ))}
      </aside>

      <div className="lmain">
        <div className="lhead">
          <h2>Biblioteca</h2>
          <span className="cnt">
            {visible.length} jogo{visible.length === 1 ? "" : "s"}
          </span>
          <label className="search">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
              <circle cx="11" cy="11" r="7" />
              <path d="M20 20l-3.5-3.5" />
            </svg>
            <input
              id="library-search"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Buscar por nome ou serial"
              aria-label="Buscar"
            />
          </label>
          <button className="btn small" onClick={onAddFolder} title="Jogos de PS3 em pasta (com PS3_GAME dentro)">
            + Pasta de PS3
          </button>
          <button className="btn play small" onClick={onAdd}>
            + Adicionar jogo
          </button>
        </div>

        {groups.length === 0 && <p className="muted">Nenhum jogo encontrado para “{query}”.</p>}
        {groups.map(([system, list]) => (
          <div className="group" key={system}>
            <h3>
              <i className="dot" style={{ background: SYSTEMS[system].dot }} />
              {SYSTEMS[system].name}
              {list.every((g) => g.demo) && <code>exemplos</code>}
            </h3>
            <div className="grid">
              {list.map((g) => (
                <button
                  key={g.id}
                  className={`card${g.id === selected?.id ? " sel" : ""}`}
                  onClick={() => onSelect(g)}
                  onDoubleClick={() => onLaunch(g)}
                >
                  <div className="cover-box">
                    <FittedCover game={g} />
                  </div>
                  <b>{g.title}</b>
                  <small>
                    <i className={g.playtime ? "" : "never"} />
                    {g.serial ?? g.format}
                  </small>
                </button>
              ))}
            </div>
          </div>
        ))}
      </div>

      {selected && <DetailPanel game={selected} onLaunch={onLaunch} onRemove={onRemove} onCover={onCover} />}
    </section>
  );
}

function DetailPanel({
  game,
  onLaunch,
  onRemove,
  onCover,
}: {
  game: Game;
  onLaunch: (g: Game) => void;
  onRemove: (g: Game) => void;
  onCover: (g: Game, kind: "web" | "file") => void;
}) {
  const colors = { "--c1": game.c1, "--c2": game.c2 } as CSSProperties;
  const sys = SYSTEMS[game.system];
  return (
    <aside className="panel">
      <div className="banner" style={colors}>
        {game.coverUrl ? (
          <img className="banner-art" src={game.coverUrl} alt="" draggable={false} />
        ) : (
          <div className={`m m-${game.motif}`} style={{ left: "40%", width: "50%" }} />
        )}
        <div className="banner-cover">
          <FittedCover game={game} align="start" />
        </div>
      </div>
      <h2>{game.title}</h2>
      <div className="tags left">
        {game.demo && <span className="tag demo">Exemplo</span>}
        {game.serial && <span className="tag mono">{game.serial}</span>}
        {game.region && <span className="tag">{game.region}</span>}
        <span className="tag fmt">{game.format}</span>
      </div>
      <div className="prow">
        <button className="btn play" onClick={() => onLaunch(game)}>
          <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <path d="M7 4.5v15l13-7.5z" />
          </svg>
          Jogar
        </button>
        <button className="btn">Rodar com…</button>
      </div>
      <dl>
        <dt>Console</dt>
        <dd>{sys.name}</dd>
        <dt>Core</dt>
        <dd>
          {sys.core} <span className="muted">(padrão)</span>
        </dd>
        <dt>Formato</dt>
        <dd>{game.format}</dd>
        <dt>Tamanho</dt>
        <dd>{game.size}</dd>
        <dt>Tempo jogado</dt>
        <dd>{game.playtime ?? "Nunca jogado"}</dd>
        <dt>Última vez</dt>
        <dd>{game.lastPlayed ?? "—"}</dd>
        <dt>Arquivo</dt>
        <dd className="path">{game.path ?? `D:\\Jogos\\${sys.folder}\\${game.file}`}</dd>
      </dl>
      <div className="sub">Jogo</div>
      {game.demo ? (
        <div className="mini">
          <button>Configurações</button>
          <button>Controles</button>
          <button>Trapaças</button>
          <button>
            Troféus {game.trophies[0]}/{game.trophies[1]}
          </button>
        </div>
      ) : (
        <div className="mini">
          <button onClick={() => onCover(game, "file")}>Trocar capa…</button>
          <button onClick={() => onCover(game, "web")}>Buscar capa na web</button>
          <button>Configurações</button>
          <button className="danger" onClick={() => onRemove(game)}>
            Remover da estante
          </button>
        </div>
      )}
      {game.playtime && game.demo && (
        <>
          <div className="sub">Save states</div>
          <div className="states" style={colors}>
            <div className="state">
              <div />
              <p>Slot 1 · ontem</p>
            </div>
            <div className="state">
              <div style={{ opacity: 0.55 }} />
              <p>Slot 2 · 3d</p>
            </div>
            <div className="state">
              <div style={{ opacity: 0.35 }} />
              <p>Auto</p>
            </div>
          </div>
        </>
      )}
    </aside>
  );
}
