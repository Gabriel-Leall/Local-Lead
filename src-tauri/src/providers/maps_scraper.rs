use crate::domain::DiscoveredPlace;
use serde::{Deserialize, Serialize};

/// Subset do JSON do gosom/google-maps-scraper (`-json`).
/// Campos reais: title, category, address/complete_address, website, phone,
/// review_rating, review_count, latitude, longitude, place_id/cid/data_id,
/// link, emails. Campos desconhecidos são ignorados.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ScraperRecord {
    #[serde(default, alias = "title")]
    pub name: Option<String>,
    #[serde(default, alias = "complete_address", alias = "completeAddress")]
    pub address: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub categories: Option<Vec<String>>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default, alias = "web_site")]
    pub website: Option<String>,
    #[serde(default, alias = "review_rating", alias = "reviewRating")]
    pub rating: Option<f64>,
    #[serde(default, alias = "review_count", alias = "reviewCount")]
    pub reviews: Option<i64>,
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    #[serde(default, alias = "placeId")]
    pub place_id: Option<String>,
    #[serde(default)]
    pub cid: Option<String>,
    #[serde(default, alias = "dataId")]
    pub data_id: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub emails: Option<serde_json::Value>,
}

impl ScraperRecord {
    pub fn primary_email(&self) -> Option<String> {
        match &self.emails {
            Some(serde_json::Value::String(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
            Some(serde_json::Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str()).find(|s| !s.trim().is_empty()).map(|s| s.trim().to_string()),
            _ => None,
        }
    }
}

fn non_empty(v: &Option<String>) -> Option<String> {
    v.clone().filter(|s| !s.trim().is_empty())
}

pub fn adapt_record(r: &ScraperRecord, idx: usize) -> Option<DiscoveredPlace> {
    let name = non_empty(&r.name)?;
    let external_id = non_empty(&r.place_id)
        .or_else(|| non_empty(&r.data_id))
        .or_else(|| non_empty(&r.cid))
        .or_else(|| non_empty(&r.link))
        .unwrap_or_else(|| format!("scraper-{idx}-{}", name.to_lowercase().replace(' ', "-")));
    let category = non_empty(&r.category).or_else(|| {
        r.categories
            .clone()
            .unwrap_or_default()
            .into_iter()
            .find(|s| !s.trim().is_empty())
    });
    Some(DiscoveredPlace {
        external_id,
        name,
        category,
        latitude: r.latitude,
        longitude: r.longitude,
        address: r.address.clone(),
        provider: "maps_scraper".into(),
        phone: r.phone.clone(),
        website: r.website.clone(),
        rating: r.rating,
        review_count: r.reviews,
    })
}

/// Aceita array JSON, objeto único ou JSONL (um objeto por linha).
pub fn parse_records(json: &str) -> Result<Vec<ScraperRecord>, String> {
    if let Ok(arr) = serde_json::from_str::<Vec<ScraperRecord>>(json) {
        return Ok(arr);
    }
    let lines: Vec<ScraperRecord> = json
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    if !lines.is_empty() {
        return Ok(lines);
    }
    if let Ok(single) = serde_json::from_str::<ScraperRecord>(json) {
        if single.name.is_some() {
            return Ok(vec![single]);
        }
    }
    Err("JSON do scraper inválido — esperado array ou JSONL de negócios".into())
}

/// Localiza o binário: primeiro no PATH, depois no backup
/// `%LOCALAPPDATA%\LocalLead\bin\google-maps-scraper.exe`.
pub fn resolve_binary() -> Result<String, String> {
    let probe = std::process::Command::new("google-maps-scraper")
        .arg("--help")
        .output();
    match probe {
        Ok(o) if o.status.success() => return Ok("google-maps-scraper".into()),
        Ok(o) => {
            return Err(format!(
                "programa respondeu {o} — reinstale o binário",
                o = o.status
            ))
        }
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            return Err(format!("falha ao chamar o scraper: {e}"))
        }
        Err(_) => {}
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let p = std::path::PathBuf::from(format!(
            "{local}\\LocalLead\\bin\\google-maps-scraper.exe"
        ));
        if p.is_file() {
            return Ok(p.to_string_lossy().into_owned());
        }
    }
    Err("google-maps-scraper não encontrado no PATH — instale ou use Importar JSON".into())
}

pub fn check_binary() -> Result<String, String> {
    resolve_binary().map(|b| format!("scraper pronto ({b})"))
}

/// Monta os argumentos do subprocesso. Query vai no arquivo de entrada
/// (`query em cidade`) e a posição em `-geo`/`-radius`/`-zoom`.
/// Usa `-fast-mode` (HTTP direto, até ~21 resultados por busca): é o modo
/// rápido e confiável — o modo navegador é lento e vive sendo bloqueado.
pub fn build_args(
    geo: &str,
    zoom: i32,
    radius_meters: f64,
    input_path: &str,
    results_path: &str,
    with_email: bool,
    concurrency: u32,
) -> Vec<String> {
    let mut args = vec![
        "-input".into(),
        input_path.into(),
        "-results".into(),
        results_path.into(),
        "-json".into(),
        "-fast-mode".into(),
        "-lang".into(),
        "pt".into(),
        "-geo".into(),
        geo.into(),
        "-zoom".into(),
        zoom.to_string(),
        "-radius".into(),
        radius_meters.round().to_string(),
        "-c".into(),
        concurrency.to_string(),
        "-exit-on-inactivity".into(),
        "3m".into(),
    ];
    if with_email {
        args.push("-email".into());
    }
    args
}

/// Zoom a partir do raio: área maior → zoom menor.
pub fn zoom_for_radius(radius_km: f64) -> i32 {
    if radius_km <= 5.0 {
        15
    } else if radius_km <= 10.0 {
        14
    } else if radius_km <= 30.0 {
        12
    } else {
        11
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapts_minimal_record() {
        let r = ScraperRecord { name: Some("Smile Center".into()), phone: Some("+1".into()), ..Default::default() };
        let d = adapt_record(&r, 0).unwrap();
        assert_eq!(d.provider, "maps_scraper");
        assert!(d.external_id.starts_with("scraper-"));
    }

    #[test]
    fn keeps_place_id_when_present() {
        let r = ScraperRecord { name: Some("X".into()), place_id: Some("ChIJ1".into()), ..Default::default() };
        assert_eq!(adapt_record(&r, 0).unwrap().external_id, "ChIJ1");
    }

    #[test]
    fn parses_real_shape_snake_case() {
        let json = r#"[{"title":"Clínica Sorriso","category":"Dentist","complete_address":"Rua A, Picos","phone":"+558900000000","website":"https://ex.com","review_rating":4.8,"review_count":120,"latitude":-7.08,"longitude":-41.46,"place_id":"ChIJ9","emails":["a@ex.com"]}]"#;
        let recs = parse_records(json).unwrap();
        assert_eq!(recs.len(), 1);
        let d = adapt_record(&recs[0], 0).unwrap();
        assert_eq!(d.name, "Clínica Sorriso");
        assert_eq!(d.rating, Some(4.8));
        assert_eq!(d.review_count, Some(120));
        assert_eq!(recs[0].primary_email().as_deref(), Some("a@ex.com"));
    }

    #[test]
    fn parses_jsonl() {
        let json = "{\"title\":\"A\"}\n{\"title\":\"B\",\"review_rating\":4.5}";
        assert_eq!(parse_records(json).unwrap().len(), 2);
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_records("not json").is_err());
    }

    #[test]
    fn builds_args_with_geo() {
        let args = build_args("-7.08,-41.46", 14, 5000.0, "in.txt", "out.json", false, 2);
        assert!(args.contains(&"-geo".to_string()));
        assert!(args.contains(&"-7.08,-41.46".to_string()));
        assert!(args.contains(&"-json".to_string()));
        assert!(args.contains(&"-fast-mode".to_string()));
        assert!(!args.contains(&"-email".to_string()));
    }

    #[test]
    fn adapts_real_fast_mode_shape() {
        let json = r#"{"title":"Araruna Odontologia","categories":["Clínica odontológica","Dentista"],"address":"Praça Josino Ferreira, 168 - Centro, Picos - PI","web_site":"https://ararunaodontologia.com/","phone":"(89)98117-8571","review_rating":4.9,"review_count":1691,"latitude":-7.0835773,"longitude":-41.4698006,"data_id":"0x79c112393149357:0x27627c7f7e452d8","place_id":"","cid":"","link":""}"#;
        let rec: ScraperRecord = serde_json::from_str(json).unwrap();
        let d = adapt_record(&rec, 0).unwrap();
        assert_eq!(d.category.as_deref(), Some("Clínica odontológica"));
        assert_eq!(d.website.as_deref(), Some("https://ararunaodontologia.com/"));
        assert_eq!(d.external_id, "0x79c112393149357:0x27627c7f7e452d8");
        assert_eq!(d.review_count, Some(1691));
    }

    #[test]
    fn zoom_shrinks_with_radius() {
        assert!(zoom_for_radius(3.0) > zoom_for_radius(50.0));
    }
}
