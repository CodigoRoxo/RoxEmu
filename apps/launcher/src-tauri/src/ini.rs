//! Editor simples de arquivos .ini que preserva tudo que não foi mexido (comentários, ordem,
//! outras seções). Suporta chaves repetidas, que o PCSX2 usa para várias teclas no mesmo botão.

pub struct Ini {
    lines: Vec<String>,
}

fn section_name(line: &str) -> Option<&str> {
    let t = line.trim();
    t.strip_prefix('[')?.strip_suffix(']')
}

fn key_of(line: &str) -> Option<&str> {
    let t = line.trim_start();
    if t.starts_with(';') || t.starts_with('#') {
        return None;
    }
    t.split_once('=').map(|(k, _)| k.trim())
}

impl Ini {
    pub fn parse(text: &str) -> Self {
        Self {
            lines: text.lines().map(str::to_string).collect(),
        }
    }

    /// Intervalo [início, fim) das linhas de conteúdo da seção (sem o cabeçalho).
    fn section_range(&self, section: &str) -> Option<(usize, usize)> {
        let header = self
            .lines
            .iter()
            .position(|l| section_name(l) == Some(section))?;
        let end = self.lines[header + 1..]
            .iter()
            .position(|l| section_name(l).is_some())
            .map_or(self.lines.len(), |p| header + 1 + p);
        Some((header + 1, end))
    }

    pub fn get(&self, section: &str, key: &str) -> Option<String> {
        let (start, end) = self.section_range(section)?;
        self.lines[start..end]
            .iter()
            .find(|l| key_of(l) == Some(key))
            .and_then(|l| l.split_once('=').map(|(_, v)| v.trim().to_string()))
    }

    /// Troca todas as linhas `key = ...` da seção por uma linha para cada valor.
    pub fn set_all(&mut self, section: &str, key: &str, values: &[&str]) {
        let new_lines: Vec<String> = values.iter().map(|v| format!("{key} = {v}")).collect();
        let Some((start, end)) = self.section_range(section) else {
            if self.lines.last().is_some_and(|l| !l.trim().is_empty()) {
                self.lines.push(String::new());
            }
            self.lines.push(format!("[{section}]"));
            self.lines.extend(new_lines);
            return;
        };
        let existing: Vec<usize> = (start..end)
            .filter(|&i| key_of(&self.lines[i]) == Some(key))
            .collect();
        // Insere onde estava a primeira ocorrência (ou no fim da seção, antes das linhas vazias).
        let insert_at = existing.first().copied().unwrap_or_else(|| {
            let mut i = end;
            while i > start && self.lines[i - 1].trim().is_empty() {
                i -= 1;
            }
            i
        });
        for &i in existing.iter().rev() {
            self.lines.remove(i);
        }
        for (n, line) in new_lines.into_iter().enumerate() {
            self.lines.insert(insert_at + n, line);
        }
    }

    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        self.set_all(section, key, &[value]);
    }

    pub fn to_text(&self) -> String {
        let mut s = self.lines.join("\r\n");
        s.push_str("\r\n");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn troca_valor_e_preserva_resto() {
        let mut ini = Ini::parse("[UI]\nA = 1\nB = 2\n\n[Pad1]\nCross = Keyboard/K\n");
        ini.set("UI", "A", "9");
        ini.set("UI", "C", "3");
        ini.set_all("Pad1", "Cross", &["Keyboard/K", "SDL-0/A"]);
        ini.set("Novo", "X", "y");
        let t = ini.to_text();
        assert!(t.contains("A = 9\r\nB = 2\r\nC = 3\r\n\r\n[Pad1]"));
        assert!(t.contains("Cross = Keyboard/K\r\nCross = SDL-0/A"));
        assert!(t.ends_with("[Novo]\r\nX = y\r\n"));
        assert_eq!(ini.get("UI", "B").as_deref(), Some("2"));
    }
}
