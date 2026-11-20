import { useMemo, useState, type CSSProperties } from "react";
import { Cover } from "../components/Cover";
import { SYSTEMS, type Game, type SystemId } from "../data/games";

interface Props {
  games: Game[];
  selectedId: number | null;
  onSelect: (game: Game) => void;
  onLaunch: (game: Game) => void;
}

/** Modo Biblioteca: grade agrupada por console e painel de detalhes. */
export function LibraryView({ games, selectedId, onSelect, onLaunch }: Props) {
  const [query, setQuery] = useState("");

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return games.filter((g) => !q || g.title.toLowerCase().includes(q) || g.serial.toLowerCase().includes(q));
  }, [games, query]);

  const groups = useMemo(() => {
    const by = new Map<SystemId, Game[]>();
    for (const g of visible) by.set(g.system, [...(by.get(g.system) ?? []), g]);
    return [...by.entries()];
  }, [visible]);

  const selected = visible.find((g) => g.id === selectedId) ?? visible[0] ?? null;

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
        </div>

        {groups.length === 0 && <p className="muted">Nenhum jogo encontrado para “{query}”.</p>}
        {groups.map(([system, list]) => (
          <div className="group" key={system}>
            <h3>
              <i className="dot" style={{ background: SYSTEMS[system].dot }} />
              {SYSTEMS[system].name} <code>D:\Jogos\{SYSTEMS[system].folder}</code>
            </h3>
            <div className="grid">
              {list.map((g) => (
                <button
                  key={g.id}
                  className={`card${g.id === selected?.id ? " sel" : ""}`}
                  onClick={() => onSelect(g)}
                  onDoubleClick={() => onLaunch(g)}
                >
                  <Cover game={g} />
                  <b>{g.title}</b>
                  <small>
                    <i className={g.playtime ? "" : "never"} />
                    {g.serial}
                  </small>
                </button>
              ))}
            </div>
          </div>
        ))}
      </div>

      {selected && <DetailPanel game={selected} onLaunch={onLaunch} />}
    </section>
  );
}

function DetailPanel({ game, onLaunch }: { game: Game; onLaunch: (g: Game) => void }) {
  const colors = { "--c1": game.c1, "--c2": game.c2 } as CSSProperties;
  const sys = SYSTEMS[game.system];
  return (
    <aside className="panel">
      <div className="banner" style={colors}>
        <div className={`m m-${game.motif}`} style={{ left: "40%", width: "50%" }} />
        <Cover game={game} />
      </div>
      <h2>{game.title}</h2>
      <div className="tags left">
        <span className="tag mono">{game.serial}</span>
        <span className="tag">{game.region}</span>
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
        <dd className="path">
          D:\Jogos\{sys.folder}\{game.file}
        </dd>
      </dl>
      <div className="sub">Jogo</div>
      <div className="mini">
        <button>Configurações</button>
        <button>Controles</button>
        <button>Trapaças</button>
        <button>
          Troféus {game.trophies[0]}/{game.trophies[1]}
        </button>
      </div>
      {game.playtime && (
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
