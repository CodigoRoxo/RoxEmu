import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Ambient } from "./components/Ambient";
import { LaunchOverlay, type LaunchSession } from "./components/LaunchOverlay";
import { CATEGORIES, SAMPLE_GAMES, fromLibrary, type Game } from "./data/games";
import { useGamepad, type PadButton } from "./hooks/useGamepad";
import * as backend from "./lib/backend";
import type { AppInfo, LibraryGame } from "./lib/backend";
import { ConsoleView } from "./views/ConsoleView";
import { LibraryView } from "./views/LibraryView";

type Mode = "console" | "library";

export default function App() {
  const [library, setLibrary] = useState<LibraryGame[]>([]);
  const [demoFavs, setDemoFavs] = useState<Set<number>>(new Set());
  const [mode, setMode] = useState<Mode>("console");
  const [category, setCategory] = useState(0);
  const [selected, setSelected] = useState(0);
  const [libraryId, setLibraryId] = useState<number | null>(null);
  const [session, setSession] = useState<LaunchSession | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const clock = useClock();

  const refresh = useCallback(async () => {
    try {
      setLibrary(await backend.listGames());
      setInfo(await backend.getAppInfo());
    } catch (e) {
      setToast(String(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // Jogos reais primeiro; os exemplos só aparecem enquanto a estante está vazia.
  const games = useMemo<Game[]>(() => {
    if (library.length) return library.map(fromLibrary);
    return SAMPLE_GAMES.map((g) => ({ ...g, favorite: g.favorite !== demoFavs.has(g.id) }));
  }, [library, demoFavs]);

  const list = useMemo(() => {
    const filtered = games.filter(CATEGORIES[category].filter);
    return filtered.length ? filtered : games;
  }, [games, category]);

  const current = list[Math.min(selected, list.length - 1)];

  // ---------- eventos do runner ----------
  const sessionRef = useRef(session);
  sessionRef.current = session;
  useEffect(() => {
    const off = backend.onRunnerEvent((ev) => {
      const s = sessionRef.current;
      if (!s || s.game.id !== ev.game_id) return;
      if (ev.kind === "started") setSession({ ...s, phase: "playing" });
      else if (ev.kind === "exited") {
        setSession(null);
        refresh();
      } else {
        setSession({ ...s, phase: "error", message: ev.message });
        refresh();
      }
    });
    return () => {
      off.then((f) => f());
    };
  }, [refresh]);

  // ---------- ações ----------
  const launch = useCallback(async (game: Game) => {
    if (game.demo) {
      setSession({ game, phase: "demo" });
      return;
    }
    setSession({ game, phase: "starting" });
    try {
      await backend.launchGame(game.id);
    } catch (e) {
      setSession({ game, phase: "error", message: String(e) });
    }
  }, []);

  const addGame = useCallback(async () => {
    try {
      const added = await backend.addGame();
      if (!added) return;
      setSession(null);
      setCategory(0);
      await refresh();
      setLibraryId(added.id);
      setToast(`${added.title} entrou na estante`);
    } catch (e) {
      setToast(String(e));
    }
  }, [refresh]);

  // Quando um jogo novo entra, seleciona ele no carrossel.
  useEffect(() => {
    if (libraryId === null) return;
    const i = list.findIndex((g) => g.id === libraryId);
    if (i >= 0) setSelected(i);
  }, [libraryId, list]);

  const removeGame = useCallback(
    async (game: Game) => {
      try {
        await backend.removeGame(game.id);
        setToast(`${game.title} saiu da estante (o arquivo continua no disco)`);
        setSelected(0);
        await refresh();
      } catch (e) {
        setToast(String(e));
      }
    },
    [refresh],
  );

  const toggleFavorite = useCallback(
    async (game: Game) => {
      if (game.demo) {
        setDemoFavs((s) => {
          const n = new Set(s);
          if (n.has(game.id)) n.delete(game.id);
          else n.add(game.id);
          return n;
        });
        return;
      }
      await backend.setFavorite(game.id, !game.favorite).catch((e) => setToast(String(e)));
      refresh();
    },
    [refresh],
  );

  const changeCategory = useCallback((delta: number) => {
    setCategory((c) => (c + delta + CATEGORIES.length) % CATEGORIES.length);
    setSelected(0);
  }, []);
  const move = useCallback(
    (delta: number) => setSelected((s) => Math.max(0, Math.min(list.length - 1, s + delta))),
    [list.length],
  );
  const openDetails = useCallback((game: Game) => {
    setLibraryId(game.id);
    setMode("library");
  }, []);

  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(null), 4500);
    return () => clearTimeout(t);
  }, [toast]);

  // ---------- teclado e controle ----------
  // Enquanto o jogo abre ou roda, a estante ignora o controle (ele é do jogo).
  const busy = session?.phase === "starting" || session?.phase === "playing";

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || busy) return;
      if (session) {
        if (e.key === "Escape" || e.key === "Backspace") setSession(null);
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
  }, [busy, session, mode, current, move, launch, changeCategory]);

  useGamepad((b: PadButton) => {
    if (busy) return;
    if (session) {
      if (b === "b") setSession(null);
      if (b === "a" && session.phase === "demo") addGame();
      return;
    }
    if (b === "lb") return changeCategory(-1);
    if (b === "rb") return changeCategory(1);
    if (b === "start") return setMode((m) => (m === "console" ? "library" : "console"));
    if (b === "x") return addGame();
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
          <button className="add" onClick={addGame} title="Adicionar jogo (X no controle)">
            + Jogo
          </button>
          <div className="seg" role="tablist" aria-label="Modo de exibição">
            <button
              role="tab"
              aria-selected={mode === "console"}
              className={mode === "console" ? "on" : ""}
              onClick={() => setMode("console")}
            >
              Console
            </button>
            <button
              role="tab"
              aria-selected={mode === "library"}
              className={mode === "library" ? "on" : ""}
              onClick={() => setMode("library")}
            >
              Biblioteca
            </button>
          </div>
          {info && !info.ps1_bios && (
            <button className="bios-warn hide-s" onClick={() => backend.openBiosFolder()} title={info.data.bios}>
              BIOS do PS1 faltando
            </button>
          )}
          <span>{clock}</span>
          <div className="av" title={info ? `ROXemu ${info.version} · dados em ${info.data.root}` : "ROXemu"}>
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
        <LibraryView
          games={list}
          selectedId={libraryId}
          onSelect={(g) => setLibraryId(g.id)}
          onLaunch={launch}
          onAdd={addGame}
          onRemove={removeGame}
        />
      )}

      <footer className="hints">
        <div>
          <span><b className="k a">A</b>Jogar</span>
          <span><b className="k y">Y</b>Detalhes</span>
          <span><b className="k x">X</b>Adicionar jogo</span>
          <span><b className="k b">B</b>Favoritar</span>
        </div>
        <div className="r">
          <span><b className="k">LB</b><b className="k">RB</b>Categoria</span>
          <span><b className="k">≡</b>Trocar modo</span>
        </div>
      </footer>

      {toast && (
        <div className="toast" role="status" onClick={() => setToast(null)}>
          {toast}
        </div>
      )}

      {session && (
        <LaunchOverlay
          session={session}
          onClose={() => setSession(null)}
          onOpenBios={() => backend.openBiosFolder().catch((e) => setToast(String(e)))}
          onAdd={addGame}
        />
      )}
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
