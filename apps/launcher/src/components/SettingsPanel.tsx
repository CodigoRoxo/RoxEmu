import { useEffect, useRef, useState } from "react";
import * as backend from "../lib/backend";
import type { SettingsView } from "../lib/backend";
import type { PadPref, PadStyle } from "../hooks/usePadStyle";
import { IosSpinner } from "./Cover";
import { PadGlyph } from "./PadGlyph";

interface Props {
  onClose: () => void;
  onSaved: (message: string) => void;
  soundOn: boolean;
  onToggleSound: () => void;
  padPref: PadPref;
  padDetected: PadStyle | null;
  onPadPref: (p: PadPref) => void;
}

const PAD_OPTIONS: [PadPref, string][] = [
  ["auto", "Automático"],
  ["ps", "PlayStation"],
  ["xbox", "Xbox"],
];

/** Configurações do ROXemu: botões do controle, artes em alta resolução (SteamGridDB) e sons. */
export function SettingsPanel({ onClose, onSaved, soundOn, onToggleSound, padPref, padDetected, onPadPref }: Props) {
  const [view, setView] = useState<SettingsView | null>(null);
  const [ps3, setPs3] = useState<backend.Ps3Status | null>(null);
  const [ps3Busy, setPs3Busy] = useState(false);

  const onSavedRef = useRef(onSaved);
  onSavedRef.current = onSaved;

  useEffect(() => {
    backend.ps3Status().then(setPs3).catch(() => {});
    const off = backend.onPs3Firmware((version) => {
      setPs3Busy(false);
      backend.ps3Status().then(setPs3).catch(() => {});
      onSavedRef.current(
        version
          ? `Firmware do PS3 ${version} instalado! Os jogos de PS3 já podem ser abertos.`
          : "A janela do RPCS3 fechou, mas o firmware não aparece instalado. Tente de novo e espere o 'Success' antes de fechar.",
      );
    });
    return () => {
      off.then((f) => f());
    };
  }, []);

  const installFirmware = async () => {
    try {
      if (await backend.installPs3Firmware()) setPs3Busy(true);
    } catch (e) {
      setError(String(e));
    }
  };
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
          <h3>Botões na tela</h3>
          <p>
            Os símbolos seguem o controle conectado
            {padDetected ? ` (agora: ${padDetected === "ps" ? "PlayStation" : "Xbox"})` : " (nenhum controle detectado)"}.
            Como no PS5: <PadGlyph action="confirm" style="ps" /> confirma e <PadGlyph action="back" style="ps" /> volta.
          </p>
          <div className="seg big" role="radiogroup" aria-label="Estilo dos botões">
            {PAD_OPTIONS.map(([value, label]) => (
              <button
                key={value}
                role="radio"
                aria-checked={padPref === value}
                className={padPref === value ? "on" : ""}
                onClick={() => onPadPref(value)}
              >
                {label}
              </button>
            ))}
          </div>
        </div>

        <div className="set-block">
          <h3>PlayStation 3</h3>
          <p>
            O PS3 roda no <b>RPCS3</b>, que já vem no ROXemu. Ele precisa do <b>firmware oficial do PS3</b>, que a
            própria Sony oferece de graça no site dela (o ROXemu não pode distribuir):
          </p>
          <ol>
            <li>
              Clique em <b>Baixar no site da Sony</b> e baixe o arquivo <b>PS3UPDAT.PUP</b>.
            </li>
            <li>
              Clique em <b>Instalar firmware</b> e escolha o arquivo baixado. O RPCS3 abre mostrando a instalação;
              quando aparecer <b>Success</b>, feche a janela dele.
            </li>
          </ol>
          <div className="key-status">
            {!ps3 ? (
              <span className="warn">Abra o ROXemu pelo app para ver o status.</span>
            ) : !ps3.emulator_found ? (
              <span className="warn">RPCS3 não encontrado (rode emulators/fetch-emulators.ps1).</span>
            ) : ps3.firmware ? (
              <span className="ok">Firmware {ps3.firmware} instalado</span>
            ) : (
              <span className="warn">Firmware ainda não instalado</span>
            )}
          </div>
          <div className="row-actions">
            <button className="btn small" onClick={() => backend.openPs3FirmwarePage().catch((e) => setError(String(e)))}>
              Baixar no site da Sony
            </button>
            <button className="btn play small" disabled={ps3Busy || !ps3?.emulator_found} onClick={installFirmware}>
              {ps3Busy ? "Instalando… feche o RPCS3 ao terminar" : "Instalar firmware"}
            </button>
          </div>
          <p className="fine">
            Jogos de PS3: adicione a <b>pasta</b> do jogo (Biblioteca → + Pasta de PS3) ou a <b>ISO</b>. A ISO precisa
            estar decifrada ou ter o arquivo .dkey ao lado.
          </p>
        </div>

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
