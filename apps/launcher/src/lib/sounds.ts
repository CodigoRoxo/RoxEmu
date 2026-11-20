// Sons de interface do ROXemu, sintetizados na hora com Web Audio (nenhum arquivo de áudio).
// Identidade: notas em Mi maior, timbre de "sino de vidro" (seno + harmônico suave) e um eco
// curto, para soar macio e "roxo" sem cansar quando se navega rápido.

export type UiSound = "move" | "nav" | "select" | "back" | "launch" | "error" | "fav" | "pop" | "boot";

const STORAGE_KEY = "rox.sound";

interface Prefs {
  enabled: boolean;
  volume: number; // 0..1
}

function loadPrefs(): Prefs {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return { enabled: true, volume: 0.6, ...JSON.parse(raw) };
  } catch {
    // sem localStorage: usa o padrão
  }
  return { enabled: true, volume: 0.6 };
}

let prefs = loadPrefs();
let ctx: AudioContext | null = null;
let master: GainNode | null = null;
let wet: GainNode | null = null;
let lastMove = 0;

function savePrefs() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
  } catch {
    // ignora
  }
}

/** Liga o áudio (o navegador só deixa depois de um clique ou tecla). */
function ensure(): AudioContext | null {
  if (!prefs.enabled) return null;
  if (!ctx) {
    try {
      ctx = new AudioContext();
    } catch {
      return null;
    }
    master = ctx.createGain();
    master.gain.value = prefs.volume * 0.5;
    master.connect(ctx.destination);

    // Eco curto com filtro: dá o "ar" de console sem virar reverb de catedral.
    const delay = ctx.createDelay(1);
    delay.delayTime.value = 0.11;
    const feedback = ctx.createGain();
    feedback.gain.value = 0.28;
    const tone = ctx.createBiquadFilter();
    tone.type = "lowpass";
    tone.frequency.value = 2600;
    wet = ctx.createGain();
    wet.gain.value = 0.35;
    wet.connect(delay);
    delay.connect(tone);
    tone.connect(feedback);
    feedback.connect(delay);
    tone.connect(master);
  }
  if (ctx.state === "suspended") ctx.resume().catch(() => {});
  return ctx;
}

const NOTE = (n: number) => 440 * Math.pow(2, (n - 69) / 12); // número MIDI -> Hz
const E5 = 76, Gs5 = 80, B5 = 83, E6 = 88, Gs6 = 92, B6 = 95, E4 = 64, B4 = 71, Gs4 = 68, A3 = 57, F3 = 53;

interface Tone {
  freq: number;
  at?: number; // segundos depois de agora
  dur?: number;
  gain?: number;
  type?: OscillatorType;
  glideTo?: number; // frequência final (desliza)
  attack?: number;
  harmonic?: number; // ganho do harmônico de oitava (brilho)
  echo?: boolean;
}

function tone(ac: AudioContext, t: Tone) {
  if (!master || !wet) return;
  const start = ac.currentTime + (t.at ?? 0);
  const dur = t.dur ?? 0.12;
  const peak = t.gain ?? 0.2;
  const attack = t.attack ?? 0.004;

  const env = ac.createGain();
  env.gain.setValueAtTime(0.0001, start);
  env.gain.exponentialRampToValueAtTime(peak, start + attack);
  env.gain.exponentialRampToValueAtTime(0.0001, start + dur);
  env.connect(master);
  if (t.echo !== false) env.connect(wet);

  const voices: [OscillatorType, number, number][] = [[t.type ?? "sine", 1, 1]];
  if (t.harmonic) voices.push(["sine", 2, t.harmonic]);
  for (const [type, mult, level] of voices) {
    const osc = ac.createOscillator();
    osc.type = type;
    osc.frequency.setValueAtTime(t.freq * mult, start);
    if (t.glideTo) osc.frequency.exponentialRampToValueAtTime(t.glideTo * mult, start + dur * 0.8);
    const g = ac.createGain();
    g.gain.value = level;
    osc.connect(g);
    g.connect(env);
    osc.start(start);
    osc.stop(start + dur + 0.05);
  }
}

const PATCHES: Record<UiSound, (ac: AudioContext) => void> = {
  // Navegar: um "tic" seco e baixinho (sem eco, sem deslizar), para não cansar quando se
  // passa por muitos jogos seguidos. Duas camadas: o corpo (seno) e um estalo curtíssimo.
  move: (ac) => {
    tone(ac, { freq: 2093, dur: 0.028, gain: 0.05, attack: 0.002, echo: false }); // C7
    tone(ac, { freq: 4186, dur: 0.012, gain: 0.018, attack: 0.001, echo: false, type: "triangle" }); // brilho
  },
  // Troca de categoria / modo: dois tiques.
  nav: (ac) => {
    tone(ac, { freq: NOTE(B5), dur: 0.06, gain: 0.1 });
    tone(ac, { freq: NOTE(E6), dur: 0.08, gain: 0.1, at: 0.05 });
  },
  // Confirmar: intervalo de quinta, brilhante.
  select: (ac) => {
    tone(ac, { freq: NOTE(E6), dur: 0.16, gain: 0.16, harmonic: 0.3 });
    tone(ac, { freq: NOTE(B6), dur: 0.22, gain: 0.14, harmonic: 0.3, at: 0.06 });
  },
  // Voltar: descida suave.
  back: (ac) => {
    tone(ac, { freq: NOTE(B5), dur: 0.1, gain: 0.12 });
    tone(ac, { freq: NOTE(E5), dur: 0.16, gain: 0.12, at: 0.06 });
  },
  // Abrir um jogo: arpejo que sobe + nota longa por baixo.
  launch: (ac) => {
    [E5, Gs5, B5, E6, Gs6].forEach((n, i) =>
      tone(ac, { freq: NOTE(n), at: i * 0.07, dur: 0.35, gain: 0.13, harmonic: 0.35 }),
    );
    tone(ac, { freq: NOTE(E4), dur: 1.1, gain: 0.1, attack: 0.25, type: "triangle", echo: false });
  },
  // Erro: grave, curto, sem ser agressivo.
  error: (ac) => {
    tone(ac, { freq: NOTE(A3), dur: 0.14, gain: 0.16, type: "triangle", echo: false });
    tone(ac, { freq: NOTE(F3), dur: 0.24, gain: 0.16, type: "triangle", at: 0.12, echo: false });
  },
  // Favoritar: brilho rápido.
  fav: (ac) => {
    [B5, E6, Gs6, B6].forEach((n, i) => tone(ac, { freq: NOTE(n), at: i * 0.035, dur: 0.12, gain: 0.08 }));
  },
  // Aviso na tela.
  pop: (ac) => tone(ac, { freq: NOTE(Gs5), glideTo: NOTE(B5), dur: 0.12, gain: 0.1, harmonic: 0.2 }),
  // Ligar o ROXemu: acorde que cresce e brilha.
  boot: (ac) => {
    [E4, B4, Gs4 + 12, E5].forEach((n, i) =>
      tone(ac, { freq: NOTE(n), at: i * 0.12, dur: 1.6, gain: 0.08, attack: 0.35, harmonic: 0.15 }),
    );
    [E6, Gs6, B6].forEach((n, i) => tone(ac, { freq: NOTE(n), at: 0.55 + i * 0.09, dur: 0.5, gain: 0.05 }));
  },
};

export function play(sound: UiSound) {
  const ac = ensure();
  if (!ac) return;
  // Navegação rápida (segurar a seta): não empilha mais de um tique a cada 45 ms.
  if (sound === "move") {
    const now = performance.now();
    if (now - lastMove < 35) return;
    lastMove = now;
  }
  try {
    PATCHES[sound](ac);
  } catch {
    // áudio indisponível: interface continua funcionando em silêncio
  }
}

export function soundEnabled(): boolean {
  return prefs.enabled;
}

export function setSoundEnabled(enabled: boolean) {
  prefs = { ...prefs, enabled };
  savePrefs();
  if (!enabled && ctx) ctx.suspend().catch(() => {});
  if (enabled) play("select");
}

/** Toca o som de boot assim que o navegador permitir áudio (no primeiro clique ou tecla). */
export function playBootWhenAllowed() {
  const tryNow = () => {
    const ac = ensure();
    if (!ac) return;
    if (ac.state === "running") {
      play("boot");
      return true;
    }
    return false;
  };
  if (tryNow()) return;
  const once = () => {
    window.removeEventListener("pointerdown", once);
    window.removeEventListener("keydown", once);
    tryNow();
  };
  window.addEventListener("pointerdown", once);
  window.addEventListener("keydown", once);
}
