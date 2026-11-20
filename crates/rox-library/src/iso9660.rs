//! Leitor mínimo de ISO9660: só o suficiente para achar e ler o `SYSTEM.CNF` na raiz do disco.
//!
//! Funciona com imagens de 2048 bytes por setor (`.iso` de PS1/PS2) e com imagens cruas de
//! 2352 bytes (`.bin` de PS1, Mode 2 ou Mode 1).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const PVD_LBA: u64 = 16;

struct Image {
    file: File,
    sector_size: u64,
    data_offset: u64,
}

impl Image {
    fn open(path: &Path) -> Option<Self> {
        // (tamanho do setor, onde começam os dados dentro do setor)
        for (sector_size, data_offset) in [(2048, 0), (2352, 24), (2352, 16)] {
            let mut img = Image {
                file: File::open(path).ok()?,
                sector_size,
                data_offset,
            };
            if let Some(pvd) = img.sector(PVD_LBA) {
                if pvd[0] == 1 && &pvd[1..6] == b"CD001" {
                    return Some(img);
                }
            }
        }
        None
    }

    fn sector(&mut self, lba: u64) -> Option<[u8; 2048]> {
        let mut buf = [0u8; 2048];
        self.file
            .seek(SeekFrom::Start(lba * self.sector_size + self.data_offset))
            .ok()?;
        self.file.read_exact(&mut buf).ok()?;
        Some(buf)
    }

    fn read_extent(&mut self, lba: u64, len: usize) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(len);
        let mut lba = lba;
        while out.len() < len {
            let s = self.sector(lba)?;
            let take = (len - out.len()).min(2048);
            out.extend_from_slice(&s[..take]);
            lba += 1;
        }
        Some(out)
    }
}

fn u32_le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

/// Procura um nome dentro de um diretório. Retorna (setor, tamanho, é diretório).
fn find_entry(
    img: &mut Image,
    dir_lba: u64,
    dir_len: usize,
    wanted: &str,
) -> Option<(u64, usize, bool)> {
    let dir = img.read_extent(dir_lba, dir_len.min(64 * 2048))?;
    let wanted = wanted.to_ascii_uppercase();
    let mut pos = 0;
    while pos < dir.len() {
        let len = dir[pos] as usize;
        if len == 0 {
            // Registros não atravessam setores: pula para o próximo setor.
            pos = (pos / 2048 + 1) * 2048;
            continue;
        }
        if pos + len > dir.len() || len < 34 {
            break;
        }
        let rec = &dir[pos..pos + len];
        let name_len = rec[32] as usize;
        let name = String::from_utf8_lossy(&rec[33..(33 + name_len).min(len)]).to_ascii_uppercase();
        // Arquivos vêm como "NOME.EXT;1"; diretórios, só "NOME".
        let bare = name.split(';').next().unwrap_or("");
        if bare == wanted {
            let lba = u32_le(&rec[2..6]) as u64;
            let size = u32_le(&rec[10..14]) as usize;
            let is_dir = rec[25] & 0x02 != 0;
            return Some((lba, size, is_dir));
        }
        pos += len;
    }
    None
}

/// Lê um arquivo da imagem pelo caminho (ex.: `["PS3_GAME", "PARAM.SFO"]`), até `max` bytes.
pub fn read_file(path: &Path, components: &[&str], max: usize) -> Option<Vec<u8>> {
    let mut img = Image::open(path)?;
    let pvd = img.sector(PVD_LBA)?;
    let root = &pvd[156..156 + 34];
    let mut lba = u32_le(&root[2..6]) as u64;
    let mut len = u32_le(&root[10..14]) as usize;
    for (i, part) in components.iter().enumerate() {
        let (e_lba, e_len, is_dir) = find_entry(&mut img, lba, len, part)?;
        let last = i + 1 == components.len();
        if last != !is_dir {
            return None; // esperava diretório e achou arquivo (ou o contrário)
        }
        lba = e_lba;
        len = e_len;
    }
    img.read_extent(lba, len.min(max))
}

/// Lê o texto do `SYSTEM.CNF` da raiz do disco (PS1/PS2), se a imagem for ISO9660.
pub fn read_system_cnf(path: &Path) -> Option<String> {
    let data = read_file(path, &["SYSTEM.CNF"], 8 * 2048)?;
    Some(String::from_utf8_lossy(&data).into_owned())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Monta uma ISO mínima (2048 bytes/setor) com SYSTEM.CNF na raiz.
    pub fn fake_iso(cnf: &str) -> Vec<u8> {
        let mut iso = vec![0u8; 2048 * 24];
        let pvd = 16 * 2048;
        iso[pvd] = 1;
        iso[pvd + 1..pvd + 6].copy_from_slice(b"CD001");
        // registro da raiz no PVD: diretório no setor 20, 2048 bytes
        let root = pvd + 156;
        iso[root] = 34;
        iso[root + 2..root + 6].copy_from_slice(&20u32.to_le_bytes());
        iso[root + 10..root + 14].copy_from_slice(&2048u32.to_le_bytes());
        // diretório raiz: um registro "SYSTEM.CNF;1" apontando para o setor 22
        let dir = 20 * 2048;
        let name = b"SYSTEM.CNF;1";
        let len = 33 + name.len() + 1;
        iso[dir] = len as u8;
        iso[dir + 2..dir + 6].copy_from_slice(&22u32.to_le_bytes());
        iso[dir + 10..dir + 14].copy_from_slice(&(cnf.len() as u32).to_le_bytes());
        iso[dir + 32] = name.len() as u8;
        iso[dir + 33..dir + 33 + name.len()].copy_from_slice(name);
        iso[22 * 2048..22 * 2048 + cnf.len()].copy_from_slice(cnf.as_bytes());
        iso
    }

    /// ISO com um arquivo dentro de uma pasta: /PS3_GAME/PARAM.SFO
    pub fn fake_iso_in_dir(dir_name: &str, file_name: &str, content: &[u8]) -> Vec<u8> {
        let mut iso = vec![0u8; 2048 * 26];
        let pvd = 16 * 2048;
        iso[pvd] = 1;
        iso[pvd + 1..pvd + 6].copy_from_slice(b"CD001");
        let root = pvd + 156;
        iso[root] = 34;
        iso[root + 2..root + 6].copy_from_slice(&20u32.to_le_bytes());
        iso[root + 10..root + 14].copy_from_slice(&2048u32.to_le_bytes());
        let put = |iso: &mut Vec<u8>, at: usize, name: &[u8], lba: u32, size: u32, dir: bool| {
            let len = 33 + name.len() + (name.len() + 1) % 2;
            iso[at] = len as u8;
            iso[at + 2..at + 6].copy_from_slice(&lba.to_le_bytes());
            iso[at + 10..at + 14].copy_from_slice(&size.to_le_bytes());
            iso[at + 25] = if dir { 0x02 } else { 0 };
            iso[at + 32] = name.len() as u8;
            iso[at + 33..at + 33 + name.len()].copy_from_slice(name);
            len
        };
        put(&mut iso, 20 * 2048, dir_name.as_bytes(), 21, 2048, true);
        let file = format!("{file_name};1");
        put(
            &mut iso,
            21 * 2048,
            file.as_bytes(),
            22,
            content.len() as u32,
            false,
        );
        iso[22 * 2048..22 * 2048 + content.len()].copy_from_slice(content);
        iso
    }

    #[test]
    fn le_arquivo_dentro_de_pasta() {
        let path = std::env::temp_dir().join(format!("rox-iso-dir-{}.iso", std::process::id()));
        std::fs::write(
            &path,
            fake_iso_in_dir("PS3_GAME", "PARAM.SFO", b"\0PSFteste"),
        )
        .unwrap();
        let data = read_file(&path, &["PS3_GAME", "PARAM.SFO"], 1024).unwrap();
        assert!(data.starts_with(b"\0PSF"));
        assert!(read_file(&path, &["PS3_GAME", "NADA.BIN"], 1024).is_none());
        assert!(read_file(&path, &["PARAM.SFO"], 1024).is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn le_system_cnf_de_uma_iso() {
        let path = std::env::temp_dir().join(format!("rox-iso-{}.iso", std::process::id()));
        std::fs::write(
            &path,
            fake_iso("BOOT2 = cdrom0:\\SLUS_203.12;1\r\nVER = 1.00\r\n"),
        )
        .unwrap();
        let cnf = read_system_cnf(&path).unwrap();
        assert!(cnf.starts_with("BOOT2"));
        let _ = std::fs::remove_file(&path);
    }
}
