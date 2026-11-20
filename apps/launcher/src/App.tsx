import { useCallback, useEffect, useMemo, useState } from "react";
import { Ambient } from "./components/Ambient";
import { LaunchOverlay } from "./components/LaunchOverlay";
import { CATEGORIES, SAMPLE_GAMES, type Game } from "./data/games";
import { useGamepad, type PadButton } from "./hooks/useGamepad";
import { getAppInfo, type AppInfo } from "./lib/backend";
import { ConsoleView } from "./views/ConsoleView";
import { LibraryView } from "./views/LibraryView";

type Mode = "console" | "library";

export default function App() {
  const [games, setGames] = useState<Game[]>(SAMPLE_GAMES);
  const [mode, setMode] = useState<Mode>("console");
  const [category, setCategory] = useState(0);
  const [selected, setSelected] = useState(0);
  const [libraryId, setLibraryId] = useState<number | null>(null);
  const [launching, setLaunching] = useState<{ game: Game; resume: boolean } | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const clock = useClock();

  useEffect(() => {
    getAppInfo().then(setInfo).catch((e) => console.error("app_info falhou", e));
  }, []);

  const list = useMemo(() => {
    const filtered = games.filter(CATEGORIES[category].filter);
    return filtered.length ? filtered : games;
  }, [games, category]);

  const current = list[Math.min(selected, list.length - 1)];

  const changeCategory = useCallback((delta: number) => {
    setCategory((c) => (c + delta + CATEGORIES.length) % CATEGORIES.length);
    setSelected(0);
  }, []);
  const move = useCallback((delta: number) => setSelected((s) => Math.max(0, Math.min(list.length - 1, s + delta))), [list.length]);
  const launch = useCallback((game: Game, resume = false) => setLaunching({ game, resume }), []);
  const toggleFavorite = useCallback(
    (game: Game) => setGames((gs) => gs.map((g) => (g.id === game.id ? { ...g, favorite: !g.favorite } : g))),
    [],
  );
  const openDetails = useCallback((game: Game) => {
    setLibraryId(game.id);
    setMode("library");
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement) return;
      if (launching) {
        if (e.key === "Escape" || e.key === "Backspace") setLaunching(null);
        return;
      }
      if (e.key === "q" || e.key === "Q") changeCategory(-1);
      else if (e.key === "e" || e.key === "E") changeCategory(1);
      if (mode !== "console" || !current) return;
      if (e.key === "ArrowRight") move(1);
      else if (e.key === "ArrowLeft") move(-1);
      else if (e.key === "Enter") launch(current);
      else return;
      e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [launching, mode, current, move, launch, changeCategory]);

  useGamepad((b: PadButton) => {
    if (launching) {
      if (b === "b") setLaunching(null);
      return;
    }
    if (b === "lb") return changeCategory(-1);
    if (b === "rb") return changeCategory(1);
    if (b === "start") return setMode((m) => (m === "console" ? "library" : "console"));
    if (mode !== "console" || !current) return;
    if (b === "right") move(1);
    else if (b === "left") move(-1);
    else if (b === "a") launch(current);
    else if (b === "y") openDetails(current);
    else if (b === "b") toggleFavorite(current);
  });

  return (
    <div className="app">
      <Ambient c1={current?.c1 ?? "#3b1a7a"} c2={current?.c2 ?? "#ff4fa3"} />

      <header className="top">
        <div className="logo">
          ROX<b>emu</b>
        </div>
        <nav className="chips" aria-label="Categorias">
          {CATEGORIES.map((c, i) => (
            <button
              key={c.label}
              className={`chip${i === category ? " on" : ""}`}
              onClick={() => {
                setCategory(i);
                setSelected(0);
              }}
            >
              {c.label}
              {i === 0 && <span className="n">{games.length}</span>}
            </button>
          ))}
        </nav>
        <div className="status">
          <div className="seg" role="tablist" aria-label="Modo de exibição">
            <button role="tab" aria-selected={mode === "console"} className={mode === "console" ? "on" : ""} onClick={() => setMode("console")}>
              Console
            </button>
            <button role="tab" aria-selected={mode === "library"} className={mode === "library" ? "on" : ""} onClick={() => setMode("library")}>
              Biblioteca
            </button>
          </div>
          <span className="tro hide-s">🏆 1.240</span>
          <span className="hide-s" title={info ? `Dados em ${info.data.root}` : "Rodando fora do app (navegador)"}>
            v{info?.version ?? "dev"}
          </span>
          <span>{clock}</span>
          <div className="av" title="Perfil">
            R
          </div>
        </div>
      </header>

      {mode === "console" ? (
        <ConsoleView
          games={list}
          selected={Math.min(selected, list.length - 1)}
          onSelect={setSelected}
          onLaunch={launch}
          onDetails={openDetails}
          onToggleFavorite={toggleFavorite}
        />
      ) : (
        <LibraryView games={list} selectedId={libraryId} onSelect={(g) => setLibraryId(g.id)} onLaunch={launch} />
      )}

      <footer className="hints">
        <div>
          <span><b className="k a">A</b>Jogar</span>
          <span><b className="k y">Y</b>Detalhes</span>
          <span><b className="k x">X</b>Configurações</span>
          <span><b className="k b">B</b>Favoritar</span>
        </div>
        <div className="r">
          <span><b className="k">LB</b><b className="k">RB</b>Categoria</span>
          <span><b className="k">≡</b>Trocar modo</span>
        </div>
      </footer>

      {launching && <LaunchOverlay game={launching.game} resume={launching.resume} onClose={() => setLaunching(null)} />}
    </div>
  );
}

function useClock() {
  const fmt = () => new Date().toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
  const [now, setNow] = useState(fmt);
  useEffect(() => {
    const t = setInterval(() => setNow(fmt()), 15_000);
    return () => clearInterval(t);
  }, []);
  return now;
}
