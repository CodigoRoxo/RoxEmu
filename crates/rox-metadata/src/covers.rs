//! Busca de capas no libretro-thumbnails (gratuito, sem chave de API).
//!
//! 1. Baixa (uma vez por semana) a lista de capas do console e guarda em cache.
//! 2. Acha a melhor capa para o jogo: nome do arquivo igual, ou mesmo título na região certa.
//! 3. Baixa a imagem e guarda em `covers/<console>/`.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rox_common::SystemId;

use crate::{thumbnail_file_name, thumbnail_url, ArtKind};

const INDEX_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600);
const PNG_MAGIC: &[u8] = b"\x89PNG";

#[derive(Debug, thiserror::Error)]
pub enum CoverError {
    #[error("falha de rede: {0}")]
    Http(String),
    #[error("resposta inesperada do servidor: {0}")]
    BadResponse(String),
    #[error("falha ao gravar no disco: {0}")]
    Io(#[from] std::io::Error),
    #[error("capa não existe no servidor")]
    NotFound,
}

impl From<ureq::Error> for CoverError {
    fn from(e: ureq::Error) -> Self {
        CoverError::Http(e.to_string())
    }
}

pub struct CoverFetcher {
    agent: ureq::Agent,
    cache_dir: PathBuf,
}

impl CoverFetcher {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        let config = ureq::Agent::config_builder()
            .user_agent("ROXemu")
            .timeout_global(Some(Duration::from_secs(30)))
            .http_status_as_error(false)
            .build();
        Self {
            agent: ureq::Agent::new_with_config(config),
            cache_dir: cache_dir.into(),
        }
    }

    /// Baixa a capa do jogo. `Ok(None)` = nenhuma capa encontrada para ele.
    pub fn fetch_boxart(
        &self,
        system: SystemId,
        file_stem: &str,
        title: &str,
        region: Option<&str>,
    ) -> Result<Option<PathBuf>, CoverError> {
        let dir = self.cache_dir.join(system.thumbnails_repo());
        std::fs::create_dir_all(&dir)?;

        let names = match self.index(system) {
            Ok(names) => names,
            Err(e) => {
                // Sem a lista (GitHub fora do ar, limite da API...): tenta adivinhar o nome.
                log::warn!("lista de capas indisponível ({e}); tentando pelo nome do arquivo");
                return self.fetch_by_guess(system, &dir, file_stem, title, region);
            }
        };
        let Some(name) = best_match(&names, file_stem, title, region) else {
            log::info!("nenhuma capa para {title}");
            return Ok(None);
        };
        let out = dir.join(format!("{name}.png"));
        if !out.exists() {
            let bytes = self.download_art(system, &name)?;
            std::fs::write(&out, bytes)?;
            log::info!("capa baixada: {}", out.display());
        }
        Ok(Some(out))
    }

    /// Plano B sem a lista: tenta o nome do arquivo e "Título (Região)".
    fn fetch_by_guess(
        &self,
        system: SystemId,
        dir: &Path,
        file_stem: &str,
        title: &str,
        region: Option<&str>,
    ) -> Result<Option<PathBuf>, CoverError> {
        let mut guesses = vec![thumbnail_file_name(file_stem)];
        let region_name = match region {
            Some("NTSC-U") => Some("USA"),
            Some("PAL") => Some("Europe"),
            Some("NTSC-J") => Some("Japan"),
            _ => None,
        };
        if let Some(r) = region_name {
            guesses.push(thumbnail_file_name(&format!("{title} ({r})")));
        }
        guesses.dedup();
        let mut last_err = None;
        for name in guesses {
            let out = dir.join(format!("{name}.png"));
            if out.exists() {
                return Ok(Some(out));
            }
            match self.download_art(system, &name) {
                Ok(bytes) => {
                    std::fs::write(&out, bytes)?;
                    log::info!("capa baixada (sem lista): {}", out.display());
                    return Ok(Some(out));
                }
                Err(CoverError::NotFound) => {}
                Err(e) => last_err = Some(e),
            }
        }
        last_err.map_or(Ok(None), Err)
    }

    fn get_bytes(&self, url: &str, limit: u64) -> Result<(u16, Vec<u8>), CoverError> {
        let mut resp = self.agent.get(url).call()?;
        let status = resp.status().as_u16();
        let body = resp.body_mut().with_config().limit(limit).read_to_vec()?;
        Ok((status, body))
    }

    /// Baixa uma imagem. No repositório algumas capas são "atalhos" (symlinks) para outra;
    /// nesse caso o servidor devolve o caminho do arquivo real, e a gente segue o atalho.
    fn download_art(&self, system: SystemId, name: &str) -> Result<Vec<u8>, CoverError> {
        let mut name = name.to_string();
        for _ in 0..3 {
            let url = thumbnail_url(system, ArtKind::BoxArt, &name);
            let (status, body) = self.get_bytes(&url, 16 * 1024 * 1024)?;
            if status == 404 {
                return Err(CoverError::NotFound);
            }
            if status != 200 {
                return Err(CoverError::BadResponse(format!("HTTP {status} em {url}")));
            }
            if body.starts_with(PNG_MAGIC) {
                return Ok(body);
            }
            match symlink_target(&body) {
                Some(target) => name = target,
                None => {
                    return Err(CoverError::BadResponse(format!(
                        "{url} não é uma imagem PNG"
                    )))
                }
            }
        }
        Err(CoverError::BadResponse("atalhos demais seguidos".into()))
    }

    /// Lista de capas disponíveis para o console (cache local de 7 dias).
    fn index(&self, system: SystemId) -> Result<Vec<String>, CoverError> {
        // "boxarts-" (antes "index-"): a lista antiga podia vir cortada em repositórios grandes.
        let cache = self
            .cache_dir
            .join(format!("boxarts-{}.json", system.thumbnails_repo()));
        let fresh = std::fs::metadata(&cache)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .is_some_and(|age| age < INDEX_MAX_AGE);
        if fresh {
            if let Some(names) = read_index(&cache) {
                return Ok(names);
            }
        }

        // Pede só a pasta Named_Boxarts: a lista "recursiva" do repositório inteiro vem cortada
        // (ou dá erro 500) nos repositórios grandes, como o do PS3.
        let repo = system.thumbnails_repo();
        let fetched = self
            .get_json_retry(&format!(
                "https://api.github.com/repos/libretro-thumbnails/{repo}/git/trees/master"
            ))
            .and_then(|root| {
                boxarts_sha(&root).ok_or_else(|| {
                    CoverError::BadResponse("repositório de capas sem Named_Boxarts".into())
                })
            })
            .and_then(|sha| {
                self.get_json_retry(&format!(
                    "https://api.github.com/repos/libretro-thumbnails/{repo}/git/trees/{sha}"
                ))
            })
            .map(|tree| parse_names(&tree));
        match fetched {
            Ok(names) => {
                std::fs::create_dir_all(&self.cache_dir)?;
                std::fs::write(&cache, serde_json::to_vec(&names).unwrap_or_default())?;
                Ok(names)
            }
            // Sem internet ou limite da API: usa o cache antigo, se houver.
            Err(e) => read_index(&cache).ok_or(e),
        }
    }
}

impl CoverFetcher {
    /// GET de JSON com até 3 tentativas quando o GitHub responde 5xx.
    fn get_json_retry(&self, url: &str) -> Result<serde_json::Value, CoverError> {
        let mut last = CoverError::BadResponse("não baixado".into());
        for attempt in 0..3u64 {
            if attempt > 0 {
                std::thread::sleep(Duration::from_secs(attempt * 2));
            }
            match self.get_bytes(url, 64 * 1024 * 1024) {
                Ok((200, body)) => {
                    return serde_json::from_slice(&body)
                        .map_err(|e| CoverError::BadResponse(e.to_string()))
                }
                Ok((status, _)) if status >= 500 => {
                    log::warn!("lista de capas: HTTP {status}, tentando de novo");
                    last = CoverError::BadResponse(format!("HTTP {status} ao listar capas"));
                }
                Ok((status, _)) => {
                    return Err(CoverError::BadResponse(format!(
                        "HTTP {status} ao listar capas"
                    )))
                }
                Err(e) => last = e,
            }
        }
        Err(last)
    }
}

/// Acha o identificador (sha) da pasta Named_Boxarts na raiz do repositório.
fn boxarts_sha(root: &serde_json::Value) -> Option<String> {
    root["tree"]
        .as_array()?
        .iter()
        .find(|e| e["path"] == "Named_Boxarts" && e["type"] == "tree")
        .and_then(|e| e["sha"].as_str().map(str::to_string))
}

/// Nomes das capas (sem `.png`) dentro da pasta Named_Boxarts.
fn parse_names(tree: &serde_json::Value) -> Vec<String> {
    tree["tree"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|e| e["path"].as_str()?.strip_suffix(".png").map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn read_index(path: &Path) -> Option<Vec<String>> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Conteúdo de um symlink do git: um caminho curto como `Outro Jogo (USA).png`.
fn symlink_target(body: &[u8]) -> Option<String> {
    if body.len() > 512 {
        return None;
    }
    let text = std::str::from_utf8(body).ok()?.trim();
    let file = text.rsplit('/').next()?;
    file.strip_suffix(".png").map(str::to_string)
}

/// Título comparável: sem tags entre parênteses, minúsculo, só letras e números.
pub fn normalize_title(s: &str) -> String {
    let mut base = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' if depth > 0 => depth -= 1,
            _ if depth == 0 => base.push(c),
            _ => {}
        }
    }
    let lower = base.to_lowercase().replace('&', " and ");
    let words: Vec<String> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect();
    // "Legend of Dragoon, The" e "The Legend of Dragoon" viram a mesma coisa.
    let mut words: Vec<&str> = words.iter().map(String::as_str).collect();
    if words.last() == Some(&"the") {
        words.pop();
    }
    if words.first() == Some(&"the") {
        words.remove(0);
    }
    words.join(" ")
}

fn region_tag(region: Option<&str>) -> Option<&'static str> {
    match region? {
        "NTSC-U" => Some("(USA"),
        "PAL" => Some("(Europe"),
        "NTSC-J" => Some("(Japan"),
        _ => None,
    }
}

/// Escolhe a melhor capa da lista para o jogo.
pub fn best_match(
    names: &[String],
    file_stem: &str,
    title: &str,
    region: Option<&str>,
) -> Option<String> {
    // 1) O arquivo já tem o nome padrão (Redump/No-Intro): bate direto.
    let exact = thumbnail_file_name(file_stem);
    if let Some(n) = names.iter().find(|n| n.eq_ignore_ascii_case(&exact)) {
        return Some(n.clone());
    }

    // 2) Mesmo título; desempate por região, versão final e disco 1.
    let wanted = normalize_title(title);
    if wanted.is_empty() {
        return None;
    }
    let tag = region_tag(region);
    names
        .iter()
        .filter(|n| normalize_title(n) == wanted)
        .max_by_key(|n| {
            let mut score = 0i32;
            if tag.is_some_and(|t| n.contains(t)) {
                score += 8;
            }
            if n.contains("(USA") {
                score += 1; // sem região conhecida, prefere a capa americana
            }
            if ["(Beta", "(Demo", "(Proto", "(Sample", "(Kiosk"]
                .iter()
                .any(|b| n.contains(b))
            {
                score -= 10;
            }
            if n.contains("(Disc ") && !n.contains("(Disc 1)") {
                score -= 2;
            }
            (score, -(n.len() as i32))
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn nome_exato_ganha() {
        let n = names(&["Metal Slug X (Europe)", "Metal Slug X (USA)"]);
        assert_eq!(
            best_match(&n, "Metal Slug X (USA)", "Metal Slug X", Some("NTSC-U")).as_deref(),
            Some("Metal Slug X (USA)")
        );
    }

    #[test]
    fn titulo_com_regiao_certa() {
        let n = names(&[
            "Jogo Legal (Japan)",
            "Jogo Legal (Europe) (En,Fr,De)",
            "Jogo Legal (USA) (Demo)",
            "Jogo Legal (USA)",
        ]);
        assert_eq!(
            best_match(&n, "jogo_legal_rip", "Jogo Legal", Some("NTSC-U")).as_deref(),
            Some("Jogo Legal (USA)")
        );
        assert_eq!(
            best_match(&n, "x", "Jogo Legal", Some("PAL")).as_deref(),
            Some("Jogo Legal (Europe) (En,Fr,De)")
        );
    }

    #[test]
    fn prefere_disco_1_e_ignora_artigo() {
        let n = names(&[
            "Aventura, The (USA) (Disc 2)",
            "Aventura, The (USA) (Disc 1)",
        ]);
        assert_eq!(
            best_match(&n, "The Aventura", "The Aventura", None).as_deref(),
            Some("Aventura, The (USA) (Disc 1)")
        );
    }

    #[test]
    fn sem_capa_retorna_none() {
        let n = names(&["Outro Jogo (USA)"]);
        assert_eq!(best_match(&n, "Jogo", "Jogo", None), None);
    }

    #[test]
    fn le_atalho_do_git() {
        assert_eq!(
            symlink_target(b"Metal Slug X (USA).png").as_deref(),
            Some("Metal Slug X (USA)")
        );
        assert_eq!(symlink_target(b"\x89PNG..."), None);
    }

    #[test]
    fn le_lista_da_api_em_duas_etapas() {
        let root: serde_json::Value = serde_json::from_str(
            r#"{"tree":[{"path":"README.md","type":"blob","sha":"x"},{"path":"Named_Boxarts","type":"tree","sha":"abc123"},{"path":"Named_Snaps","type":"tree","sha":"def"}]}"#,
        )
        .unwrap();
        assert_eq!(boxarts_sha(&root).as_deref(), Some("abc123"));
        let sub: serde_json::Value = serde_json::from_str(
            r#"{"tree":[{"path":"A (USA).png"},{"path":"B (Europe).png"},{"path":".gitkeep"}]}"#,
        )
        .unwrap();
        assert_eq!(
            parse_names(&sub),
            vec!["A (USA)".to_string(), "B (Europe)".to_string()]
        );
    }
}
