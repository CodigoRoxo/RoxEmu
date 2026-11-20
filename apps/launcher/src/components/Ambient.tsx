import { useEffect, useRef } from "react";

/** Fundo roxo/rosa com brilhos que seguem as cores do jogo selecionado e partículas de luz. */
export function Ambient({ c1, c2, art }: { c1: string; c2: string; art?: string }) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const c = canvas.current;
    const ctx = c?.getContext("2d");
    if (!c || !ctx) return;
    const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const dpr = window.devicePixelRatio || 1;
    let w = 0;
    let h = 0;
    let raf = 0;
    type P = { x: number; y: number; r: number; v: number; a: number; rgb: string };
    let parts: P[] = [];

    const resize = () => {
      w = c.width = c.offsetWidth * dpr;
      h = c.height = c.offsetHeight * dpr;
      parts = Array.from({ length: 70 }, () => ({
        x: Math.random() * w,
        y: Math.random() * h,
        r: (Math.random() * 1.6 + 0.4) * dpr,
        v: (Math.random() * 0.25 + 0.05) * dpr,
        a: Math.random() * 0.5 + 0.15,
        rgb: Math.random() < 0.5 ? "236,72,153" : "167,139,250",
      }));
    };
    const draw = () => {
      ctx.clearRect(0, 0, w, h);
      for (const p of parts) {
        ctx.beginPath();
        ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(${p.rgb},${p.a})`;
        ctx.fill();
        if (!still) {
          p.y -= p.v;
          if (p.y < -5) {
            p.y = h + 5;
            p.x = Math.random() * w;
          }
        }
      }
      if (!still) raf = requestAnimationFrame(draw);
    };
    resize();
    draw();
    window.addEventListener("resize", resize);
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", resize);
    };
  }, []);

  return (
    <div className="bg" aria-hidden="true">
      {art && <img key={art} className="hero" src={art} alt="" draggable={false} />}
      <div className="blob a" style={{ backgroundColor: `color-mix(in srgb, ${c1} 45%, #7c3aed)` }} />
      <div className="blob b" style={{ backgroundColor: `color-mix(in srgb, ${c2} 40%, #db2777)` }} />
      <canvas ref={canvas} />
    </div>
  );
}
