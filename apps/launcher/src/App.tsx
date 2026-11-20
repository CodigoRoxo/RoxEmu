import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Ambient } from "./components/Ambient";
import { LaunchOverlay, type LaunchSession } from "./components/LaunchOverlay";
import { CATEGORIES, SAMPLE_GAMES, fromLibrary, type Game } from "./data/games";
import { useCoverColors } from "./hooks/useCoverColors";
import { useGamepad, type PadButton } from "./hooks/useGamepad";
import * as backend from "./lib/backend";
import type { AppInfo, LibraryGame } from "./lib/backend";
import * as sound from "./lib/sounds";
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
    sound.playBootWhenAllowed();
    // Capas baixadas em segundo plano chegam por aqui.
    const off = backend.onLibraryChanged(() => {
      backend.listGames().then(setLibrary).catch(() => {});
    });
    const offMissing = backend.onCoverMissing(({ title, error }) =>
      setToast(
        error
          ? `Não consegui buscar a capa de ${title} (${error}). Tente de novo ou escolha uma imagem em Biblioteca.`
          : `Não achei a capa de ${title} na internet. Em Biblioteca → "Trocar capa…" você escolhe uma imagem.`,
      ),
    );
    return () => {
      off.then((f) => f());
      offMissing.then((f) => f());
    };
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

  // Avisa da BIOS só para consoles que têm jogos na estante (ou PS1, com a estante vazia).
  const missingBios = useMemo(() => {
    if (!info) return [];
    return info.systems
      .filter((s) => s.requires_bios && !s.has_bios)
      .filter((s) => (library.length ? library.some((g) => g.system === s.id) : s.id === "ps1"))
      .map((s) => s.id.toUpperCase());
  }, [info, library]);
  const [tint1, tint2] = useCoverColors(current?.coverUrl, [current?.c1 ?? "#3b1a7a", current?.c2 ?? "#ff4fa3"]);

  // ---------- eventos do runner ----------
  const sessionRef = useRef(session);
  sessionRef.current = session;
  useEffect(() => {
    const off = backend.onRunnerEvent((ev) => {
      const s = sessionRef.current;
      if (!s || s.game.id !== ev.game_id) return;
      if (ev.kind === "started") setSession({ ...s, phase: "playing" });
      else if (ev.kind === "exited") {
        sound.play("back");
        setSession(null);
        refresh();
      } else {
        sound.play("error");
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
      sound.play("select");
      setSession({ game, phase: "demo" });
      return;
    }
    sound.play("launch");
    setSession({ game, phase: "starting" });
    try {
      await backend.launchGame(game.id);
    } catch (e) {
      sound.play("error");
      setSession({ game, phase: "error", message: String(e) });
    }
  }, []);

  const closeSession = useCallback(() => {
    sound.play("back");
    setSession(null);
  }, []);

  const addGame = useCallback(async () => {
    try {
      const added = await backend.addGame();
      if (!added) return;
      sound.play("select");
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
        sound.play("back");
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
      sound.play(game.favorite ? "back" : "fav");
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
    sound.play("nav");
    setCategory((c) => (c + delta + CATEGORIES.length) % CATEGORIES.length);
    setSelected(0);
  }, []);
  const pickCategory = useCallback((i: number) => {
    sound.play("nav");
    setCategory(i);
    setSelected(0);
  }, []);
  const selectIndex = useCallback(
    (i: number) => {
      const next = Math.max(0, Math.min(list.length - 1, i));
      if (next !== selected) sound.play("move");
      setSelected(next);
    },
    [list.length, selected],
  );
  const move = useCallback((delta: number) => selectIndex(selected + delta), [selectIndex, selected]);
  const switchMode = useCallback((m: Mode) => {
    sound.play("nav");
    setMode(m);
  }, []);
  const openDetails = useCallback((game: Game) => {
    sound.play("select");
    setLibraryId(game.id);
    setMode("library");
  }, []);
  const [soundOn, setSoundOn] = useState(sound.soundEnabled());

  useEffect(() => {
    if (toast) sound.play("pop");
  }, [toast]);

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
        if (e.key === "Escape" || e.key === "Backspace") closeSession();
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
  }, [busy, session, mode, current, move, launch, changeCategory, closeSession]);

  useGamepad((b: PadButton) => {
    if (busy) return;
    if (session) {
      if (b === "b") closeSession();
      if (b === "a" && session.phase === "demo") addGame();
      return;
    }
    if (b === "lb") return changeCategory(-1);
    if (b === "rb") return changeCategory(1);
    if (b === "start") return switchMode(mode === "console" ? "library" : "console");
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
      <Ambient c1={tint1} c2={tint2} art={current?.coverUrl} />

      <header className="top">
        <div className="logo">
          ROX<b>emu</b>
        </div>
        <nav className="chips" aria-label="Categorias">
          {CATEGORIES.map((c, i) => (
            <button
              key={c.label}
              className={`chip${i === category ? " on" : ""}`}
              onClick={() => pickCategory(i)}
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
              onClick={() => switchMode("console")}
            >
              Console
            </button>
            <button
              role="tab"
              aria-selected={mode === "library"}
              className={mode === "library" ? "on" : ""}
              onClick={() => switchMode("library")}
            >
              Biblioteca
            </button>
          </div>
          {missingBios.length > 0 && (
            <button
              className="bios-warn hide-s"
              onClick={() => backend.openBiosFolder()}
              title={`Coloque a BIOS em ${info?.data.bios}`}
            >
              BIOS do {missingBios.join(" e ")} faltando
            </button>
          )}
          <button
            className={`sound${soundOn ? "" : " off"}`}
            aria-label={soundOn ? "Desligar sons" : "Ligar sons"}
            title={soundOn ? "Desligar sons" : "Ligar sons"}
            onClick={() => {
              sound.setSoundEnabled(!soundOn);
              setSoundOn(!soundOn);
            }}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true">
              <path d="M4 9h4l5-4v14l-5-4H4z" fill="currentColor" stroke="none" />
              {soundOn ? <path d="M16 9a4 4 0 010 6M18.5 6.5a8 8 0 010 11" /> : <path d="M16 9l5 6M21 9l-5 6" />}
            </svg>
          </button>
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
          onSelect={selectIndex}
          onLaunch={launch}
          onDetails={openDetails}
          onToggleFavorite={toggleFavorite}
        />
      ) : (
        <LibraryView
          games={list}
          selectedId={libraryId}
          onSelect={(g) => {
            if (g.id !== libraryId) sound.play("move");
            setLibraryId(g.id);
          }}
          onLaunch={launch}
          onAdd={addGame}
          onRemove={removeGame}
          onCover={async (game, kind) => {
            try {
              if (kind === "web") {
                await backend.refreshCover(game.id);
                setToast(`Procurando a capa de ${game.title}…`);
              } else if (await backend.chooseCover(game.id)) {
                await refresh();
                setToast("Capa trocada");
              }
            } catch (e) {
              setToast(String(e));
            }
          }}
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
          onClose={closeSession}
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
