//! Identificação de jogos de PS1.
//!
//! Todo disco de PS1 tem um `SYSTEM.CNF` na raiz com uma linha como
//! `BOOT = cdrom:\SLUS_005.94;1`. O nome do executável é o serial do jogo,
//! que a gente normaliza para `SLUS-00594` e cruza com a base Redump.

/// Extrai o serial normalizado do conteúdo de um `SYSTEM.CNF`.
pub fn serial_from_system_cnf(text: &str) -> Option<String> {
    let boot = text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        key.trim()
            .eq_ignore_ascii_case("BOOT")
            .then(|| value.trim())
    })?;
    // "cdrom:\SLUS_005.94;1" ou "cdrom:SLUS_005.94;1" ou "cdrom:\\EXE\\SCES_011.20;1"
    let file = boot.rsplit(['\\', ':', '/']).next()?;
    let file = file.split(';').next()?;
    normalize_serial(file)
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
    fn rejeita_o_que_nao_e_serial() {
        assert_eq!(serial_from_system_cnf("BOOT = cdrom:\\MAIN.EXE;1"), None);
        assert_eq!(serial_from_system_cnf("TCB = 4"), None);
    }
}
