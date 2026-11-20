import { useEffect, useState } from "react";

/**
 * Tira duas cores marcantes da capa (uma escura e uma vibrante) para tingir o fundo da tela.
 * Se a imagem não puder ser lida (ou não houver capa), devolve as cores de reserva.
 */
export function useCoverColors(url: string | undefined, fallback: [string, string]): [string, string] {
  const [colors, setColors] = useState<[string, string] | null>(null);

  useEffect(() => {
    setColors(null);
    if (!url) return;
    let alive = true;
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => {
      try {
        const size = 24;
        const canvas = document.createElement("canvas");
        canvas.width = canvas.height = size;
        const ctx = canvas.getContext("2d", { willReadFrequently: true });
        if (!ctx) return;
        ctx.drawImage(img, 0, 0, size, size);
        const { data } = ctx.getImageData(0, 0, size, size);
        let dark = { score: -1, rgb: [40, 20, 70] };
        let vivid = { score: -1, rgb: [236, 72, 153] };
        for (let i = 0; i < data.length; i += 4) {
          const [r, g, b] = [data[i], data[i + 1], data[i + 2]];
          const max = Math.max(r, g, b);
          const min = Math.min(r, g, b);
          const sat = max === 0 ? 0 : (max - min) / max;
          const light = (max + min) / 510;
          const vividScore = sat * (1 - Math.abs(light - 0.55));
          if (vividScore > vivid.score) vivid = { score: vividScore, rgb: [r, g, b] };
          const darkScore = sat * (1 - light);
          if (darkScore > dark.score) dark = { score: darkScore, rgb: [r, g, b] };
        }
        const css = (c: number[]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
        if (alive) setColors([css(dark.rgb), css(vivid.rgb)]);
      } catch {
        // imagem sem permissão de leitura: fica com as cores de reserva
      }
    };
    img.src = url;
    return () => {
      alive = false;
    };
  }, [url]);

  return colors ?? fallback;
}
