import { useEffect, useRef } from "react";

/** Botões no layout "standard" da Gamepad API (Xbox/DualShock/DualSense via SDL do navegador). */
export type PadButton = "a" | "b" | "x" | "y" | "lb" | "rb" | "start" | "left" | "right" | "up" | "down";

const INDEX: Record<Exclude<PadButton, "left" | "right" | "up" | "down">, number> = {
  a: 0,
  b: 1,
  x: 2,
  y: 3,
  lb: 4,
  rb: 5,
  start: 9,
};

/** Chama `onPress` uma vez a cada botão apertado (com borda de subida, sem repetir enquanto segura). */
export function useGamepad(onPress: (button: PadButton) => void) {
  const handler = useRef(onPress);
  handler.current = onPress;

  useEffect(() => {
    let raf = 0;
    let prev = new Set<PadButton>();

    const tick = () => {
      const pad = navigator.getGamepads?.().find((p) => p !== null);
      const now = new Set<PadButton>();
      if (pad) {
        for (const [name, i] of Object.entries(INDEX)) {
          if (pad.buttons[i]?.pressed) now.add(name as PadButton);
        }
        const [ax = 0, ay = 0] = pad.axes;
        if (pad.buttons[14]?.pressed || ax < -0.6) now.add("left");
        if (pad.buttons[15]?.pressed || ax > 0.6) now.add("right");
        if (pad.buttons[12]?.pressed || ay < -0.6) now.add("up");
        if (pad.buttons[13]?.pressed || ay > 0.6) now.add("down");
        for (const b of now) if (!prev.has(b)) handler.current(b);
      }
      prev = now;
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);
}
