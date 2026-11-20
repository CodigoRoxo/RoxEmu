//! Escala o quadro do jogo para o tamanho da janela, mantendo a proporção da TV (4:3)
//! com faixas pretas nas laterais quando preciso.

/// Cor de fundo das faixas (roxo bem escuro do ROXemu).
const BORDER: u32 = 0x000B_0614;

pub struct Screen {
    pub buffer: Vec<u32>,
    pub width: usize,
    pub height: usize,
    xmap: Vec<usize>,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            width: 0,
            height: 0,
            xmap: Vec::new(),
        }
    }

    /// Desenha `src` (sw x sh) no centro de uma tela `dw x dh` com a proporção `aspect`.
    pub fn draw(&mut self, src: &[u32], sw: usize, sh: usize, dw: usize, dh: usize, aspect: f32) {
        let (dw, dh) = (dw.max(1), dh.max(1));
        if self.width != dw || self.height != dh {
            self.width = dw;
            self.height = dh;
            self.buffer = vec![BORDER; dw * dh];
        }
        if sw == 0 || sh == 0 || src.len() < sw * sh {
            return;
        }
        let aspect = if aspect > 0.0 {
            aspect
        } else {
            sw as f32 / sh as f32
        };

        // Área útil com a proporção certa.
        let (mut tw, mut th) = (dw, (dw as f32 / aspect) as usize);
        if th > dh {
            th = dh;
            tw = (dh as f32 * aspect) as usize;
        }
        let (tw, th) = (tw.max(1), th.max(1));
        let ox = (dw - tw) / 2;
        let oy = (dh - th) / 2;

        self.xmap.clear();
        self.xmap.extend((0..tw).map(|x| x * sw / tw));

        self.buffer.fill(BORDER);
        for y in 0..th {
            let sy = y * sh / th;
            let src_row = &src[sy * sw..sy * sw + sw];
            let dst = &mut self.buffer[(oy + y) * dw + ox..(oy + y) * dw + ox + tw];
            for (d, &sx) in dst.iter_mut().zip(&self.xmap) {
                *d = src_row[sx];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mantem_4_por_3_com_faixas() {
        let mut s = Screen::new();
        let src = vec![0x00FF_FFFF; 320 * 240];
        s.draw(&src, 320, 240, 1600, 900, 4.0 / 3.0);
        // 900 de altura -> 1200 de largura útil, faixas de 200 px em cada lado
        assert_eq!(s.buffer[450 * 1600 + 100], BORDER);
        assert_eq!(s.buffer[450 * 1600 + 800], 0x00FF_FFFF);
        assert_eq!(s.buffer[450 * 1600 + 1500], BORDER);
    }
}
