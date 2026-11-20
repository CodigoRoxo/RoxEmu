//! Artes em alta resolução do SteamGridDB (acervo comunitário e gratuito; precisa de uma chave
//! de API pessoal, que o usuário gera no site).
//!
//! - **Hero**: arte de fundo larga (1920 ou 3840 px) para a tela inteira, como na home do PS5.
//! - **Logo**: logotipo do jogo com fundo transparente, no lugar do título escrito.
//!
//! Se o jogo não tiver hero (comum em jogos antigos), procura a **mesma franquia/personagem**,
//! preferindo os lançamentos mais recentes: o "Spider-Man" do PS1 ganha a arte dos Spider-Man novos.
//! Nesse caso não usa o logo, que seria de outro jogo.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

use crate::covers::{normalize_title, CoverError};

const API: &str = "https://www.steamgriddb.com/api/v2";

/// Artes encontradas para um jogo.
#[derive(Debug, Clone, Default)]
pub struct HeroArt {
    pub hero: Option<PathBuf>,
    pub logo: Option<PathBuf>,
    /// Arte quadrada do jogo, para os ícones da fileira (como no PS5).
    pub icon: Option<PathBuf>,
    /// Capa vertical (600x900), usada quando o acervo de capas não tem a do jogo.
    pub cover: Option<PathBuf>,
    /// Nome do jogo no SteamGridDB de onde veio a arte.
    pub source_name: Option<String>,
    /// A arte é de outro jogo da franquia (plano B), não do próprio jogo.
    pub from_franchise: bool,
}

#[derive(Debug, Clone)]
struct SgdbGame {
    id: u64,
    name: String,
    release: i64,
}

pub struct SteamGridDb {
    agent: ureq::Agent,
    key: String,
    out_dir: PathBuf,
}

impl SteamGridDb {
    pub fn new(key: &str, out_dir: impl Into<PathBuf>) -> Self {
        let config = ureq::Agent::config_builder()
            .user_agent("ROXemu")
            .timeout_global(Some(Duration::from_secs(30)))
            .http_status_as_error(false)
            .build();
        Self {
            agent: ureq::Agent::new_with_config(config),
            key: key.trim().to_string(),
            out_dir: out_dir.into(),
        }
    }

    fn get_json(&self, path: &str) -> Result<Value, CoverError> {
        let url = format!("{API}{path}");
        let mut resp = self
            .agent
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.key))
            .call()?;
        let status = resp.status().as_u16();
        let body = resp
            .body_mut()
            .with_config()
            .limit(8 * 1024 * 1024)
            .read_to_vec()?;
        match status {
            200 => {
                serde_json::from_slice(&body).map_err(|e| CoverError::BadResponse(e.to_string()))
            }
            401 | 403 => Err(CoverError::BadResponse(
                "chave do SteamGridDB inválida ou expirada".into(),
            )),
            404 => Ok(Value::Null),
            s => Err(CoverError::BadResponse(format!(
                "SteamGridDB respondeu HTTP {s}"
            ))),
        }
    }

    /// Testa se a chave funciona (usado ao salvar nas configurações).
    pub fn check_key(&self) -> Result<(), CoverError> {
        self.get_json("/search/autocomplete/tetris").map(|_| ())
    }

    fn search(&self, term: &str) -> Result<Vec<SgdbGame>, CoverError> {
        let json = self.get_json(&format!("/search/autocomplete/{}", url_encode(term)))?;
        Ok(parse_search(&json))
    }

    /// URL da melhor imagem de um tipo ("heroes", "logos" ou "grids" quadradas) para o jogo.
    fn best_image(&self, kind: &str, game_id: u64) -> Result<Option<String>, CoverError> {
        let (endpoint, query) = match kind {
            "heroes" => (
                "heroes",
                "?dimensions=3840x1240,1920x620&types=static&nsfw=false&humor=false",
            ),
            "grids" => (
                "grids",
                "?dimensions=1024x1024,512x512&types=static&nsfw=false&humor=false",
            ),
            // capa vertical, no formato de caixa
            "covers" => (
                "grids",
                "?dimensions=600x900,660x930,342x482&types=static&nsfw=false&humor=false",
            ),
            _ => ("logos", "?types=static&nsfw=false&humor=false"),
        };
        let json = self.get_json(&format!("/{endpoint}/game/{game_id}{query}"))?;
        Ok(parse_first_url(&json))
    }

    fn fetch_kind(
        &self,
        kind: &str,
        game_id: u64,
        sub: &str,
        key: &str,
    ) -> Result<Option<PathBuf>, CoverError> {
        match self.best_image(kind, game_id)? {
            Some(url) => self.download(&url, sub, key).map(Some),
            None => Ok(None),
        }
    }

    fn download(&self, url: &str, sub: &str, name: &str) -> Result<PathBuf, CoverError> {
        let dir = self.out_dir.join(sub);
        std::fs::create_dir_all(&dir)?;
        let ext = url
            .rsplit('.')
            .next()
            .filter(|e| ["png", "jpg", "jpeg", "webp"].contains(&e.to_ascii_lowercase().as_str()))
            .unwrap_or("png");
        let out = dir.join(format!("{name}.{ext}"));
        if out.exists() {
            return Ok(out);
        }
        let mut resp = self.agent.get(url).call()?;
        if resp.status().as_u16() != 200 {
            return Err(CoverError::BadResponse(format!(
                "HTTP {} ao baixar arte",
                resp.status()
            )));
        }
        let bytes = resp
            .body_mut()
            .with_config()
            .limit(40 * 1024 * 1024)
            .read_to_vec()?;
        std::fs::write(&out, bytes)?;
        Ok(out)
    }

    /// Procura e baixa hero, logo e ícone quadrado. `file_key` nomeia os arquivos salvos.
    ///
    /// Logo e ícone vêm sempre do **próprio** jogo (são a "identidade" dele). Só a arte de fundo
    /// pode vir de outro jogo da franquia, quando o próprio não tem.
    pub fn fetch(&self, title: &str, file_key: &str) -> Result<HeroArt, CoverError> {
        let wanted = normalize_title(title);
        let mut tried = std::collections::HashSet::new();
        let mut art = HeroArt::default();
        let mut identity_done = false;

        for (round, term) in search_terms(title).iter().enumerate() {
            let results = self.search(term)?;
            for game in rank(&results, &wanted, round > 0).into_iter().take(4) {
                if !tried.insert(game.id) {
                    continue;
                }
                let exact = normalize_title(&game.name) == wanted;
                if exact && !identity_done {
                    identity_done = true;
                    art.logo = self.fetch_kind("logos", game.id, "logos", file_key)?;
                    art.icon = self.fetch_kind("grids", game.id, "icons", file_key)?;
                    art.cover = self.fetch_kind("covers", game.id, "sgdb-covers", file_key)?;
                }
                let Some(hero) = self.fetch_kind("heroes", game.id, "heroes", file_key)? else {
                    continue;
                };
                log::info!(
                    "arte de {title}: {} ({})",
                    game.name,
                    if exact {
                        "do próprio jogo"
                    } else {
                        "da franquia"
                    }
                );
                art.hero = Some(hero);
                art.source_name = Some(game.name);
                art.from_franchise = !exact;
                return Ok(art);
            }
        }
        Ok(art)
    }
}

/// Termos de busca, do mais específico ao mais amplo (franquia).
/// "Need for Speed - Underground 2" -> ["Need for Speed - Underground 2", "Need for Speed"].
pub fn search_terms(title: &str) -> Vec<String> {
    let mut terms = vec![title.trim().to_string()];
    for sep in [" - ", ": ", " – "] {
        if let Some((franchise, _)) = title.split_once(sep) {
            terms.push(franchise.trim().to_string());
        }
    }
    // Tira números e sequências no fim: "Tekken 3" -> "Tekken", "Final Fantasy VII" -> "Final Fantasy".
    let words: Vec<&str> = terms[terms.len() - 1].split_whitespace().collect();
    let is_sequel = |w: &str| {
        w.chars().all(|c| c.is_ascii_digit())
            || (w.len() <= 4 && w.chars().all(|c| "IVXLC".contains(c)))
    };
    if words.len() > 1 && words.last().is_some_and(|w| is_sequel(w)) {
        terms.push(words[..words.len() - 1].join(" "));
    }
    terms.dedup();
    terms
}

/// Ordena os resultados: nome idêntico primeiro; na busca da franquia, os mais recentes primeiro.
fn rank(results: &[SgdbGame], wanted: &str, franchise_round: bool) -> Vec<SgdbGame> {
    let mut list: Vec<SgdbGame> = results.to_vec();
    list.sort_by_key(|g| {
        let exact = normalize_title(&g.name) == wanted;
        (
            std::cmp::Reverse(exact),
            if franchise_round { -g.release } else { 0 },
        )
    });
    list
}

fn parse_search(json: &Value) -> Vec<SgdbGame> {
    json["data"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|g| {
                    Some(SgdbGame {
                        id: g["id"].as_u64()?,
                        name: g["name"].as_str()?.to_string(),
                        release: g["release_date"].as_i64().unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_first_url(json: &Value) -> Option<String> {
    json["data"]
        .as_array()?
        .iter()
        .find_map(|img| img["url"].as_str().map(str::to_string))
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Nome de arquivo seguro para guardar as artes de um jogo.
pub fn file_key(system: &str, title: &str) -> String {
    let clean: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{system}-{}", clean.trim())
}

/// Existe arte salva para essa chave? (evita baixar de novo)
pub fn cached(out_dir: &Path, key: &str) -> Option<PathBuf> {
    ["png", "jpg", "jpeg", "webp"]
        .iter()
        .map(|e| out_dir.join("heroes").join(format!("{key}.{e}")))
        .find(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn termos_vao_do_jogo_para_a_franquia() {
        assert_eq!(
            search_terms("Need for Speed - Underground 2"),
            vec!["Need for Speed - Underground 2", "Need for Speed"]
        );
        assert_eq!(search_terms("Tekken 3"), vec!["Tekken 3", "Tekken"]);
        assert_eq!(
            search_terms("Final Fantasy VII"),
            vec!["Final Fantasy VII", "Final Fantasy"]
        );
        assert_eq!(search_terms("Spider-Man"), vec!["Spider-Man"]);
    }

    #[test]
    fn franquia_prefere_o_mais_recente() {
        let results = vec![
            SgdbGame {
                id: 1,
                name: "Spider-Man".into(),
                release: 946_684_800,
            },
            SgdbGame {
                id: 2,
                name: "Marvel's Spider-Man 2".into(),
                release: 1_697_760_000,
            },
            SgdbGame {
                id: 3,
                name: "Marvel's Spider-Man".into(),
                release: 1_536_192_000,
            },
        ];
        // Busca do próprio jogo: o nome idêntico vem primeiro.
        assert_eq!(rank(&results, "spider man", false)[0].id, 1);
        // Rodada da franquia (sem nome idêntico): o mais novo primeiro.
        let franchise = rank(&results[1..], "spider man", true);
        assert_eq!(franchise[0].id, 2);
    }

    #[test]
    fn le_respostas_da_api() {
        let search: Value = serde_json::from_str(
            r#"{"success":true,"data":[{"id":42,"name":"Metal Slug X","release_date":954547200,"types":["steam"],"verified":true}]}"#,
        )
        .unwrap();
        let games = parse_search(&search);
        assert_eq!(games[0].id, 42);
        assert_eq!(games[0].name, "Metal Slug X");

        let heroes: Value = serde_json::from_str(
            r#"{"success":true,"data":[{"id":7,"url":"https://cdn2.steamgriddb.com/hero/abc.png","width":3840,"height":1240}]}"#,
        )
        .unwrap();
        assert_eq!(
            parse_first_url(&heroes).as_deref(),
            Some("https://cdn2.steamgriddb.com/hero/abc.png")
        );
        assert_eq!(parse_first_url(&Value::Null), None);
    }

    #[test]
    fn codifica_termo_na_url() {
        assert_eq!(url_encode("Spider-Man: 2"), "Spider-Man%3A%202");
    }
}
