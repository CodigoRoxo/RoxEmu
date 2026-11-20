import { useEffect, useState } from "react";
import * as backend from "../lib/backend";
import type { SettingsView } from "../lib/backend";
import { IosSpinner } from "./Cover";

interface Props {
  onClose: () => void;
  onSaved: (message: string) => void;
  soundOn: boolean;
  onToggleSound: () => void;
}

/** Configurações do ROXemu. Por enquanto: artes em alta resolução (SteamGridDB) e sons. */
export function SettingsPanel({ onClose, onSaved, soundOn, onToggleSound }: Props) {
  const [view, setView] = useState<SettingsView | null>(null);
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    backend.getSettings().then(setView).catch((e) => setError(String(e)));
  }, []);

  const save = async (value: string) => {
    if (value.trim() === "" && !view?.has_steamgriddb_key) {
      setError("Cole a chave gerada no site do SteamGridDB.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await backend.setSteamgriddbKey(value);
      setView(await backend.getSettings());
      setKey("");
      onSaved(
        value.trim()
          ? "Chave salva! Buscando as artes de fundo dos seus jogos…"
          : "Chave removida. A estante volta a usar as capas no fundo.",
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="settings-backdrop" onClick={onClose}>
      <section className="settings" role="dialog" aria-label="Configurações" onClick={(e) => e.stopPropagation()}>
        <header>
          <h2>Configurações</h2>
          <button className="round small" aria-label="Fechar" onClick={onClose}>
            ✕
          </button>
        </header>

        <div className="set-block">
          <h3>Artes de fundo em alta resolução</h3>
          <p>
            O ROXemu busca artes em 2K/4K e o logo de cada jogo no <b>SteamGridDB</b>, um acervo gratuito feito pela
            comunidade. Se o jogo não tiver, usa a arte de um jogo mais novo da mesma franquia. Para isso, o site pede
            uma chave pessoal (grátis):
          </p>
          <ol>
            <li>Entre em steamgriddb.com com sua conta Steam.</li>
            <li>
              Vá em <b>Preferences → API</b> e gere a chave.
            </li>
            <li>Cole a chave aqui embaixo e salve.</li>
          </ol>
          <button className="btn small" onClick={() => backend.openSteamgriddbPage().catch((e) => setError(String(e)))}>
            Abrir a página da chave
          </button>

          <div className="key-status">
            {view?.has_steamgriddb_key ? (
              <span className="ok">Chave configurada ({view.steamgriddb_key_hint})</span>
            ) : (
              <span className="warn">Sem chave: o fundo usa a capa desfocada.</span>
            )}
          </div>

          <label className="key-field">
            <input
              id="sgdb-key"
              type="password"
              autoComplete="off"
              spellCheck={false}
              value={key}
              placeholder={view?.has_steamgriddb_key ? "Colar nova chave" : "Colar a chave do SteamGridDB"}
              onChange={(e) => {
                setKey(e.target.value);
                setError(null);
              }}
              onKeyDown={(e) => e.key === "Enter" && !busy && save(key)}
            />
            <button className="btn play small" disabled={busy} onClick={() => save(key)}>
              {busy ? "Testando…" : "Salvar"}
            </button>
          </label>
          {busy && (
            <div className="key-busy">
              <IosSpinner label="Testando a chave" />
            </div>
          )}
          {error && <p className="err">{error}</p>}
          {view?.has_steamgriddb_key && (
            <button className="link danger" disabled={busy} onClick={() => save("")}>
              Remover chave
            </button>
          )}
          <p className="fine">
            A chave fica só neste computador. O ROXemu envia ao SteamGridDB apenas o nome dos jogos para achar as artes.
          </p>
        </div>

        <div className="set-block row-between">
          <div>
            <h3>Sons da interface</h3>
            <p>Os sons sintetizados da estante (navegar, abrir jogo, voltar).</p>
          </div>
          <button className={`toggle${soundOn ? " on" : ""}`} role="switch" aria-checked={soundOn} onClick={onToggleSound}>
            <i />
          </button>
        </div>
      </section>
    </div>
  );
}
