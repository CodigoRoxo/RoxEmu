//! Leitura mínima de arquivos `.cue`: só o que a biblioteca precisa para saber
//! quais `.bin` pertencem a qual jogo e quantas faixas existem.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CueSheet {
    /// Arquivos referenciados por linhas `FILE "..." BINARY`, na ordem.
    pub files: Vec<String>,
    /// Número total de linhas `TRACK`.
    pub track_count: usize,
    /// Quantas faixas são de áudio (`TRACK nn AUDIO`).
    pub audio_tracks: usize,
}

pub fn parse_cue(text: &str) -> CueSheet {
    let mut sheet = CueSheet {
        files: Vec::new(),
        track_count: 0,
        audio_tracks: 0,
    };
    for line in text.lines() {
        let line = line.trim();
        let upper = line.to_ascii_uppercase();
        if upper.starts_with("FILE ") {
            if let Some(name) = quoted_or_first_word(&line[5..]) {
                sheet.files.push(name);
            }
        } else if upper.starts_with("TRACK ") {
            sheet.track_count += 1;
            if upper.ends_with("AUDIO") {
                sheet.audio_tracks += 1;
            }
        }
    }
    sheet
}

fn quoted_or_first_word(rest: &str) -> Option<String> {
    let rest = rest.trim();
    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"')?;
        return Some(stripped[..end].to_string());
    }
    rest.split_whitespace().next().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_cue_com_faixas_de_audio() {
        let cue = r#"FILE "Jogo (USA) (Track 1).bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
FILE "Jogo (USA) (Track 2).bin" BINARY
  TRACK 02 AUDIO
    INDEX 00 00:00:00
    INDEX 01 00:02:00
"#;
        let s = parse_cue(cue);
        assert_eq!(
            s.files,
            vec!["Jogo (USA) (Track 1).bin", "Jogo (USA) (Track 2).bin"]
        );
        assert_eq!(s.track_count, 2);
        assert_eq!(s.audio_tracks, 1);
    }

    #[test]
    fn aceita_nome_sem_aspas() {
        let s = parse_cue("FILE jogo.bin BINARY\nTRACK 01 MODE2/2352\n");
        assert_eq!(s.files, vec!["jogo.bin"]);
    }
}
