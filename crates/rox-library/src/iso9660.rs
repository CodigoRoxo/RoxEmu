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

/// Lê o texto do `SYSTEM.CNF` da raiz do disco, se a imagem for ISO9660.
pub fn read_system_cnf(path: &Path) -> Option<String> {
    let mut img = Image::open(path)?;
    let pvd = img.sector(PVD_LBA)?;
    let root = &pvd[156..156 + 34];
    let root_lba = u32_le(&root[2..6]) as u64;
    let root_len = (u32_le(&root[10..14]) as usize).min(64 * 2048);
    let dir = img.read_extent(root_lba, root_len)?;

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
        if name.starts_with("SYSTEM.CNF") {
            let lba = u32_le(&rec[2..6]) as u64;
            let size = (u32_le(&rec[10..14]) as usize).min(8 * 2048);
            let data = img.read_extent(lba, size)?;
            return Some(String::from_utf8_lossy(&data).into_owned());
        }
        pos += len;
    }
    None
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
