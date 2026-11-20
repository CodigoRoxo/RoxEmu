//! Conversão dos formatos de pixel do libretro para 0RGB de 32 bits (o que a janela espera).

use crate::ffi;

/// Copia um quadro do core para `out` (0x00RRGGBB por pixel), sem o "pitch" extra das linhas.
///
/// # Segurança
/// `data` precisa apontar para `height` linhas de `pitch` bytes, como garante a API libretro.
pub(crate) unsafe fn convert_frame(
    format: u32,
    data: *const u8,
    width: usize,
    height: usize,
    pitch: usize,
    out: &mut Vec<u32>,
) {
    out.clear();
    out.reserve(width * height);
    for y in 0..height {
        let row = data.add(y * pitch);
        match format {
            ffi::RETRO_PIXEL_FORMAT_XRGB8888 => {
                let px = std::slice::from_raw_parts(row as *const u32, width);
                out.extend(px.iter().map(|p| p & 0x00FF_FFFF));
            }
            ffi::RETRO_PIXEL_FORMAT_RGB565 => {
                let px = std::slice::from_raw_parts(row as *const u16, width);
                out.extend(px.iter().map(|&p| rgb565(p)));
            }
            _ => {
                let px = std::slice::from_raw_parts(row as *const u16, width);
                out.extend(px.iter().map(|&p| rgb1555(p)));
            }
        }
    }
}

fn expand5(v: u16) -> u32 {
    let v = v as u32;
    (v << 3) | (v >> 2)
}

fn expand6(v: u16) -> u32 {
    let v = v as u32;
    (v << 2) | (v >> 4)
}

pub(crate) fn rgb565(p: u16) -> u32 {
    let r = expand5((p >> 11) & 0x1F);
    let g = expand6((p >> 5) & 0x3F);
    let b = expand5(p & 0x1F);
    (r << 16) | (g << 8) | b
}

pub(crate) fn rgb1555(p: u16) -> u32 {
    let r = expand5((p >> 10) & 0x1F);
    let g = expand5((p >> 5) & 0x1F);
    let b = expand5(p & 0x1F);
    (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branco_e_preto_em_565_e_1555() {
        assert_eq!(rgb565(0xFFFF), 0x00FF_FFFF);
        assert_eq!(rgb565(0x0000), 0);
        assert_eq!(rgb1555(0x7FFF), 0x00FF_FFFF);
        assert_eq!(rgb565(0xF800), 0x00FF_0000);
    }

    #[test]
    fn respeita_o_pitch() {
        // 2x2 pixels XRGB8888 com 4 bytes de sobra por linha
        let raw: [u32; 6] = [
            0xFF11_2233,
            0x0044_5566,
            0xDEAD,
            0x0077_8899,
            0x00AA_BBCC,
            0xBEEF,
        ];
        let mut out = Vec::new();
        unsafe {
            convert_frame(
                ffi::RETRO_PIXEL_FORMAT_XRGB8888,
                raw.as_ptr() as *const u8,
                2,
                2,
                12,
                &mut out,
            )
        };
        assert_eq!(
            out,
            vec![0x0011_2233, 0x0044_5566, 0x0077_8899, 0x00AA_BBCC]
        );
    }
}
