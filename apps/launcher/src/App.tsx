import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Ambient } from "./components/Ambient";
import { LaunchOverlay, type LaunchSession } from "./components/LaunchOverlay";
import { SettingsPanel } from "./components/SettingsPanel";
import { CATEGORIES, SAMPLE_GAMES, SYSTEMS, fromLibrary, type Game, type SystemId } from "./data/games";
import { useCoverColors } from "./hooks/useCoverColors";
import { useGamepad, type PadButton } from "./hooks/useGamepad";
import { usePadStyle } from "./hooks/usePadStyle";
import { PadGlyph } from "./components/PadGlyph";
import { gridOrder } from "./views/LibraryView";

/** Ações da interface, iguais para teclado e controle. */
type UiAction = "left" | "right" | "up" | "down" | "confirm" | "back" | "details" | "fav" | "prev" | "next" | "options" | "create";
import * as backend from "./lib/backend";
import type { AppInfo, LibraryGame } from "./lib/backend";
import * as sound from "./lib/sounds";
import { ConsoleView, type ShelfItem } from "./views/ConsoleView";
import { LibraryView } from "./views/LibraryView";

type Mode = "console" | "library";

interface Shelf {
  id: string;
  label: string;
  system?: SystemId;
}

/** Ordem das estantes de console no topo. */
const SYSTEM_ORDER: SystemId[] = ["ps1", "ps2", "snes", "gba"];

export default function App() {
  const [library, setLibrary] = useState<LibraryGame[]>([]);
  const [demoFavs, setDemoFavs] = useState<Set<number>>(new Set());
  const [coverLoading, setCoverLoading] = useState<Set<number>>(new Set());
  const [mode, setMode] = useState<Mode>("console");
  // modo Console: estante atual e posição dentro de cada estante
  const [shelfId, setShelfId] = useState("all");
  const [shelfPos, setShelfPos] = useState<Record<string, number>>({});
  const [focusId, setFocusId] = useState<number | null>(null);
  // modo Biblioteca
  const [category, setCategory] = useState(0);
  const [libraryId, setLibraryId] = useState<number | null>(null);
  const [session, setSession] = useState<LaunchSession | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [soundOn, setSoundOn] = useState(sound.soundEnabled());
  const [showSettings, setShowSettings] = useState(false);
  const { style: padStyle, pref: padPref, setPref: setPadPref, detected: padDetected } = usePadStyle();
  const clock = useClock();
  const toggleSound = useCallback(() => {
    sound.setSoundEnabled(!soundOn);
    setSoundOn(!soundOn);
  }, [soundOn]);

  const refresh = useCallback(async () => {
    try {
      setLibrary(await backend.listGames());
      setInfo(await backend.getAppInfo());
    } catch (e) {
      setToast(String(e));
    }
  }, []);

  const setLoading = useCallback((id: number, on: boolean) => {
    setCoverLoading((s) => {
      if (s.has(id) === on) return s;
      const n = new Set(s);
      if (on) n.add(id);
      else n.delete(id);
      return n;
    });
  }, []);

  useEffect(() => {
    refresh();
    sound.playBootWhenAllowed();
    // Capas buscadas em segundo plano: começa (spinner) -> chega (library-changed) ou falha.
    const offLoading = backend.onCoverLoading((id) => setLoading(id, true));
    const offChanged = backend.onLibraryChanged((id) => {
      setLoading(id, false);
      backend.listGames().then(setLibrary).catch(() => {});
    });
    const offMissing = backend.onCoverMissing(({ id, title, error }) => {
      setLoading(id, false);
      setToast(
        error
          ? `Não consegui buscar a capa de ${title} (${error}). Tente de novo ou escolha uma imagem em Biblioteca.`
          : `Não achei a capa de ${title} na internet. Em Biblioteca → "Trocar capa…" você escolhe uma imagem.`,
      );
    });
    const offArtError = backend.onArtError((msg) => setToast(`Artes de fundo: ${msg}`));
    return () => {
      for (const off of [offLoading, offChanged, offMissing, offArtError]) off.then((f) => f());
    };
  }, [refresh, setLoading]);

  // Jogos reais (os jogados por último primeiro); exemplos só com a estante vazia.
  const games = useMemo<Game[]>(() => {
    if (library.length) {
      return [...library]
        .sort((a, b) => (b.last_played ?? 0) - (a.last_played ?? 0) || b.added_at - a.added_at)
        .map((g) => ({ ...fromLibrary(g), coverLoading: coverLoading.has(g.id) }));
    }
    return SAMPLE_GAMES.map((g) => ({ ...g, favorite: g.favorite !== demoFavs.has(g.id) }));
  }, [library, demoFavs, coverLoading]);

  // ---------- estantes do modo Console ----------
  const systemsPresent = useMemo(
    () => SYSTEM_ORDER.filter((s) => games.some((g) => g.system === s)),
    [games],
  );
  const shelves = useMemo<Shelf[]>(() => {
    const list: Shelf[] = [{ id: "all", label: "Todos" }];
    for (const s of systemsPresent) list.push({ id: s, label: SYSTEMS[s].name, system: s });
    if (games.some((g) => g.favorite)) list.push({ id: "fav", label: "Favoritos" });
    return list;
  }, [systemsPresent, games]);

  const shelf = shelves.find((s) => s.id === shelfId) ?? shelves[0];
  const shelfItems = useMemo<ShelfItem[]>(() => {
    const asItems = (list: Game[]): ShelfItem[] => list.map((game) => ({ kind: "game", game }));
    if (shelf.system) return asItems(games.filter((g) => g.system === shelf.system));
    if (shelf.id === "fav") return asItems(games.filter((g) => g.favorite));
    // "Todos": os jogos e, no fim, uma pasta por console (quando há mais de um).
    const folders: ShelfItem[] =
      systemsPresent.length > 1
        ? systemsPresent.map((s) => ({ kind: "folder", system: s, games: games.filter((g) => g.system === s) }))
        : [];
    return [...asItems(games), ...folders];
  }, [shelf, games, systemsPresent]);

  const pos = Math.max(0, Math.min(shelfItems.length - 1, shelfPos[shelf.id] ?? 0));
  const item = shelfItems[pos];

  // Leva a seleção até um jogo específico (ex.: o que acabou de ser adicionado).
  useEffect(() => {
    if (focusId === null) return;
    const i = shelfItems.findIndex((it) => it.kind === "game" && it.game.id === focusId);
    if (i >= 0) {
      setShelfPos((p) => ({ ...p, [shelf.id]: i }));
      setFocusId(null);
    }
  }, [focusId, shelfItems, shelf.id]);

  // ---------- modo Biblioteca ----------
  const libraryList = useMemo(() => {
    const filtered = games.filter(CATEGORIES[category].filter);
    return filtered.length ? filtered : games;
  }, [games, category]);

  // Jogo "em foco" (dá cor e arte ao fundo).
  const focusGame: Game | undefined =
    mode === "console"
      ? item?.kind === "game"
        ? item.game
        : item?.games[0]
      : (libraryList.find((g) => g.id === libraryId) ?? libraryList[0]);

  const missingBios = useMemo(() => {
    if (!info) return [];
    return info.systems
      .filter((s) => s.requires_bios && !s.has_bios)
      .filter((s) => (library.length ? library.some((g) => g.system === s.id) : s.id === "ps1"))
      .map((s) => s.id.toUpperCase());
  }, [info, library]);
  const [tint1, tint2] = useCoverColors(focusGame?.coverUrl, [focusGame?.c1 ?? "#3b1a7a", focusGame?.c2 ?? "#ff4fa3"]);

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
      // A busca da capa já começou no backend: mostra o carregando desde já.
      if (!added.cover) setLoading(added.id, true);
      setSession(null);
      setCategory(0);
      setShelfId(added.system);
      await refresh();
      setFocusId(added.id);
      setLibraryId(added.id);
      setToast(`${added.title} entrou na estante de ${SYSTEMS[added.system].name}`);
    } catch (e) {
      setToast(String(e));
    }
  }, [refresh, setLoading]);

  const removeGame = useCallback(
    async (game: Game) => {
      try {
        await backend.removeGame(game.id);
        sound.play("back");
        setToast(`${game.title} saiu da estante (o arquivo continua no disco)`);
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

  const goShelf = useCallback((id: string) => {
    sound.play("nav");
    setShelfId(id);
  }, []);
  const changeShelf = useCallback(
    (delta: number) => {
      const i = shelves.findIndex((s) => s.id === shelf.id);
      goShelf(shelves[(i + delta + shelves.length) % shelves.length].id);
    },
    [shelves, shelf.id, goShelf],
  );
  const changeCategory = useCallback((delta: number) => {
    sound.play("nav");
    setCategory((c) => (c + delta + CATEGORIES.length) % CATEGORIES.length);
  }, []);
  const selectIndex = useCallback(
    (i: number) => {
      const next = Math.max(0, Math.min(shelfItems.length - 1, i));
      if (next !== pos) sound.play("move");
      setShelfPos((p) => ({ ...p, [shelf.id]: next }));
    },
    [shelfItems.length, pos, shelf.id],
  );
  const activate = useCallback(
    (it: ShelfItem) => {
      if (it.kind === "game") launch(it.game);
      else goShelf(it.system);
    },
    [launch, goShelf],
  );
  const switchMode = useCallback((m: Mode) => {
    sound.play("nav");
    setMode(m);
  }, []);
  const openDetails = useCallback((game: Game) => {
    sound.play("select");
    setLibraryId(game.id);
    setMode("library");
  }, []);

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

  // Navegação da Biblioteca pelo controle: anda pela grade na ordem em que ela aparece.
  const libraryNav = useCallback(
    (dir: "left" | "right" | "up" | "down") => {
      const ordered = gridOrder(libraryList);
      if (!ordered.length) return;
      const cur = Math.max(0, ordered.findIndex((g) => g.id === libraryId));
      const grid = document.querySelector<HTMLElement>(".library .grid");
      const cols = grid ? getComputedStyle(grid).gridTemplateColumns.split(" ").length : 4;
      const step = { left: -1, right: 1, up: -cols, down: cols }[dir];
      const next = Math.max(0, Math.min(ordered.length - 1, cur + step));
      if (next !== cur) sound.play("move");
      setLibraryId(ordered[next].id);
    },
    [libraryList, libraryId],
  );

  /**
   * Uma ação só para teclado e controle (botões no padrão do PS5):
   * ✕ confirma · ○ volta · △ detalhes · □ favorito · L1/R1 estante · Options modo · Create jogo.
   */
  const dispatch = (a: UiAction) => {
    if (busy) return;
    if (showSettings) {
      if (a === "back") {
        sound.play("back");
        setShowSettings(false);
      }
      return;
    }
    if (session) {
      if (a === "back" || (a === "confirm" && session.phase === "error")) closeSession();
      else if (a === "confirm" && session.phase === "demo") addGame();
      return;
    }
    if (a === "prev" || a === "next") {
      const d = a === "prev" ? -1 : 1;
      return mode === "console" ? changeShelf(d) : changeCategory(d);
    }
    if (a === "options") return switchMode(mode === "console" ? "library" : "console");
    if (a === "create") return addGame();

    if (mode === "library") {
      const sel = libraryList.find((g) => g.id === libraryId) ?? gridOrder(libraryList)[0];
      if (a === "left" || a === "right" || a === "up" || a === "down") libraryNav(a);
      else if (a === "confirm" && sel) launch(sel);
      else if (a === "fav" && sel) toggleFavorite(sel);
      else if (a === "back") switchMode("console");
      return;
    }

    if (!item) return;
    if (a === "right") selectIndex(pos + 1);
    else if (a === "left") selectIndex(pos - 1);
    else if (a === "confirm") activate(item);
    else if (a === "details" && item.kind === "game") openDetails(item.game);
    else if (a === "fav" && item.kind === "game") toggleFavorite(item.game);
    else if (a === "back") {
      // ○ volta: de uma estante de console para "Todos"; em "Todos", para o começo da fileira.
      if (shelf.id !== "all") goShelf("all");
      else if (pos > 0) selectIndex(0);
    }
  };
  const dispatchRef = useRef(dispatch);
  dispatchRef.current = dispatch;

  useEffect(() => {
    const KEYS: Record<string, UiAction> = {
      ArrowLeft: "left",
      ArrowRight: "right",
      ArrowUp: "up",
      ArrowDown: "down",
      Enter: "confirm",
      Escape: "back",
      Backspace: "back",
      q: "prev",
      e: "next",
      f: "fav",
      i: "details",
      Tab: "options",
    };
    const onKey = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLInputElement;
      if (typing && e.key !== "Escape") return;
      const a = KEYS[e.key.length === 1 ? e.key.toLowerCase() : e.key];
      if (!a) return;
      e.preventDefault();
      dispatchRef.current(a);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useGamepad((b: PadButton) => {
    const PAD: Record<PadButton, UiAction> = {
      a: "confirm",
      b: "back",
      y: "details",
      x: "fav",
      lb: "prev",
      rb: "next",
      start: "options",
      select: "create",
      left: "left",
      right: "right",
      up: "up",
      down: "down",
    };
    dispatchRef.current(PAD[b]);
  });

  return (
    <div className="app">
      <Ambient c1={tint1} c2={tint2} art={focusGame?.coverUrl} hero={focusGame?.heroUrl} />

      <header className="top">
        <div className="logo">
          ROX<b>emu</b>
        </div>
        {mode === "console" ? (
          <nav className="shelves" aria-label="Estantes">
            {shelves.map((s) => (
              <button
                key={s.id}
                className={`shelf-tab${s.id === shelf.id ? " on" : ""}`}
                onClick={() => goShelf(s.id)}
              >
                {s.system && <i style={{ background: SYSTEMS[s.system].dot }} />}
                {s.label}
              </button>
            ))}
          </nav>
        ) : (
          <nav className="chips" aria-label="Categorias">
            {CATEGORIES.map((c, i) => (
              <button
                key={c.label}
                className={`chip${i === category ? " on" : ""}`}
                onClick={() => {
                  sound.play("nav");
                  setCategory(i);
                }}
              >
                {c.label}
                {i === 0 && <span className="n">{games.length}</span>}
              </button>
            ))}
          </nav>
        )}
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
            className="gear"
            aria-label="Configurações"
            title="Configurações"
            onClick={() => {
              sound.play("select");
              setShowSettings(true);
            }}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true">
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.6 1.6 0 00.3 1.8l.1.1a2 2 0 11-2.8 2.8l-.1-.1a1.6 1.6 0 00-2.7 1.1V21a2 2 0 11-4 0v-.1a1.6 1.6 0 00-2.7-1.1l-.1.1a2 2 0 11-2.8-2.8l.1-.1A1.6 1.6 0 003 15.4H3a2 2 0 110-4h.1a1.6 1.6 0 001.1-2.7l-.1-.1a2 2 0 112.8-2.8l.1.1A1.6 1.6 0 009.6 4V3a2 2 0 114 0v.1a1.6 1.6 0 002.7 1.1l.1-.1a2 2 0 112.8 2.8l-.1.1a1.6 1.6 0 001.1 2.7H21a2 2 0 110 4h-.1a1.6 1.6 0 00-1.5 1.3z" />
            </svg>
          </button>
          <button
            className={`sound${soundOn ? "" : " off"}`}
            aria-label={soundOn ? "Desligar sons" : "Ligar sons"}
            title={soundOn ? "Desligar sons" : "Ligar sons"}
            onClick={toggleSound}
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
          items={shelfItems}
          selected={pos}
          onSelect={selectIndex}
          onActivate={activate}
          onDetails={openDetails}
          onToggleFavorite={toggleFavorite}
        />
      ) : (
        <LibraryView
          games={libraryList}
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
                setLoading(game.id, true);
                await backend.refreshCover(game.id);
              } else if (await backend.chooseCover(game.id)) {
                await refresh();
                setToast("Capa trocada");
              }
            } catch (e) {
              setLoading(game.id, false);
              setToast(String(e));
            }
          }}
        />
      )}

      <footer className="hints">
        <div>
          <span>
            <PadGlyph action="confirm" style={padStyle} />
            {item?.kind === "folder" && mode === "console" ? "Abrir pasta" : "Jogar"}
          </span>
          <span>
            <PadGlyph action="back" style={padStyle} />
            Voltar
          </span>
          {mode === "console" && (
            <span>
              <PadGlyph action="details" style={padStyle} />
              Detalhes
            </span>
          )}
          <span>
            <PadGlyph action="fav" style={padStyle} />
            Favoritar
          </span>
        </div>
        <div className="r">
          <span>
            <PadGlyph action="l1" style={padStyle} />
            <PadGlyph action="r1" style={padStyle} />
            {mode === "console" ? "Estante" : "Categoria"}
          </span>
          <span>
            <PadGlyph action="create" style={padStyle} />
            Adicionar jogo
          </span>
          <span>
            <PadGlyph action="options" style={padStyle} />
            {mode === "console" ? "Biblioteca" : "Console"}
          </span>
        </div>
      </footer>

      {toast && (
        <div className="toast" role="status" onClick={() => setToast(null)}>
          {toast}
        </div>
      )}

      {showSettings && (
        <SettingsPanel
          onClose={() => {
            sound.play("back");
            setShowSettings(false);
          }}
          onSaved={(msg) => {
            setToast(msg);
            refresh();
          }}
          soundOn={soundOn}
          onToggleSound={toggleSound}
          padPref={padPref}
          padDetected={padDetected}
          onPadPref={setPadPref}
        />
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
