//! Identificação de discos de PlayStation (PS1 e PS2).
//!
//! Todo disco tem um `SYSTEM.CNF` na raiz. No PS1 a linha é `BOOT = cdrom:\SLUS_005.94;1`;
//! no PS2 é `BOOT2 = cdrom0:\SLUS_203.12;1`. Daí sai o console e o serial do jogo, que a gente
//! normaliza para `SLUS-00594` e usa para achar nome e capa.

use rox_common::SystemId;

/// Console e serial lidos do disco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscId {
    pub system: SystemId,
    pub serial: String,
}

/// Lê o `SYSTEM.CNF` e diz de qual console é o disco e qual o serial.
pub fn disc_id_from_system_cnf(text: &str) -> Option<DiscId> {
    let mut found = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let system = match key.trim().to_ascii_uppercase().as_str() {
            "BOOT2" => SystemId::Ps2,
            "BOOT" => SystemId::Ps1,
            _ => continue,
        };
        // "cdrom:\SLUS_005.94;1", "cdrom0:\SLUS_203.12;1", "cdrom:\\EXE\\SCES_011.20;1"
        let file = value.trim().rsplit(['\\', ':', '/']).next()?;
        let file = file.split(';').next()?;
        if let Some(serial) = normalize_serial(file) {
            found = Some(DiscId { system, serial });
            if system == SystemId::Ps2 {
                break; // BOOT2 é definitivo
            }
        }
    }
    found
}

/// Só o serial (compatibilidade).
pub fn serial_from_system_cnf(text: &str) -> Option<String> {
    disc_id_from_system_cnf(text).map(|d| d.serial)
}

/// Identifica um disco: primeiro lendo o ISO9660 de verdade; se não der (ex.: imagem estranha),
/// procura a linha do `SYSTEM.CNF` nos primeiros megabytes. CHD/CSO/PBP comprimidos: `None`.
pub fn identify_disc(data_file: &std::path::Path) -> Option<DiscId> {
    if let Some(id) = crate::iso9660::read_system_cnf(data_file)
        .as_deref()
        .and_then(disc_id_from_system_cnf)
    {
        return Some(id);
    }
    use std::io::Read;
    const LIMIT: u64 = 8 * 1024 * 1024;
    let mut buf = Vec::new();
    std::fs::File::open(data_file)
        .ok()?
        .take(LIMIT)
        .read_to_end(&mut buf)
        .ok()?;
    find_boot_line(&buf)
}

/// Só o serial de um arquivo de disco (compatibilidade).
pub fn scan_serial(data_file: &std::path::Path) -> Option<String> {
    identify_disc(data_file).map(|d| d.serial)
}

fn find_boot_line(buf: &[u8]) -> Option<DiscId> {
    let mut start = 0;
    while let Some(pos) = buf[start..].windows(4).position(|w| w == b"BOOT") {
        let at = start + pos;
        let end = (at + 80).min(buf.len());
        let line: String = buf[at..end]
            .iter()
            .take_while(|&&b| b != b'\r' && b != b'\n' && b != 0)
            .map(|&b| b as char)
            .collect();
        if line.to_ascii_lowercase().contains("cdrom") {
            if let Some(id) = disc_id_from_system_cnf(&line) {
                return Some(id);
            }
        }
        start = at + 4;
    }
    None
}

/// `SLUS_005.94` -> `SLUS-00594`. Retorna `None` se não parecer um serial.
pub fn normalize_serial(raw: &str) -> Option<String> {
    let cleaned: String = raw.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let split = cleaned.find(|c: char| c.is_ascii_digit())?;
    let (prefix, digits) = cleaned.split_at(split);
    let prefix_ok =
        (3..=4).contains(&prefix.len()) && prefix.chars().all(|c| c.is_ascii_alphabetic());
    let digits_ok = digits.len() == 5 && digits.chars().all(|c| c.is_ascii_digit());
    (prefix_ok && digits_ok).then(|| format!("{}-{}", prefix.to_ascii_uppercase(), digits))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrai_serial_do_system_cnf() {
        let cnf = "BOOT = cdrom:\\SLUS_005.94;1\r\nTCB = 4\r\nEVENT = 10\r\nSTACK = 801FFFF0\r\n";
        assert_eq!(serial_from_system_cnf(cnf).as_deref(), Some("SLUS-00594"));
    }

    #[test]
    fn aceita_variacoes_de_caminho() {
        assert_eq!(
            serial_from_system_cnf("BOOT=cdrom:SCES_011.20;1").as_deref(),
            Some("SCES-01120")
        );
        assert_eq!(
            serial_from_system_cnf("boot = cdrom:\\EXE\\SLPS_022.15;1").as_deref(),
            Some("SLPS-02215")
        );
    }

    #[test]
    fn acha_o_system_cnf_no_meio_dos_bytes() {
        let mut disco = vec![0u8; 50_000];
        disco[100..104].copy_from_slice(b"BOOT"); // falso positivo sem "cdrom"
        let cnf = b"BOOT = cdrom:\\SLES_123.45;1\r\nTCB = 4\r\n";
        disco[30_000..30_000 + cnf.len()].copy_from_slice(cnf);
        assert_eq!(
            find_boot_line(&disco).map(|d| d.serial).as_deref(),
            Some("SLES-12345")
        );
    }

    #[test]
    fn boot2_e_ps2() {
        let id = disc_id_from_system_cnf(
            "BOOT2 = cdrom0:\\SLUS_203.12;1\r\nVER = 1.00\r\nVMODE = NTSC\r\n",
        )
        .unwrap();
        assert_eq!(id.system, SystemId::Ps2);
        assert_eq!(id.serial, "SLUS-20312");
        let ps1 = disc_id_from_system_cnf("BOOT = cdrom:\\SLUS_012.12;1").unwrap();
        assert_eq!(ps1.system, SystemId::Ps1);
    }

    #[test]
    fn rejeita_o_que_nao_e_serial() {
        assert_eq!(serial_from_system_cnf("BOOT = cdrom:\\MAIN.EXE;1"), None);
        assert_eq!(serial_from_system_cnf("TCB = 4"), None);
    }
}
