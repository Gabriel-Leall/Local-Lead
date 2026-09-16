use crate::domain::DiscoveredPlace;
use crate::error::AppError;
use serde::Deserialize;
use std::collections::HashMap;

const OVERPASS_URL: &str = "https://overpass-api.de/api/interpreter";

/// Nicho (pt-BR) → tags OSM + regex de nome. Inspirado no Caça-Cliente:
/// dado aberto, sem chave, sem navegador.
fn niche_rule(query: &str) -> (Vec<(&'static str, &'static str)>, String) {
    let q = query.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| q.contains(w));
    if has(&["dentista", "odonto", "ortodont"]) {
        (vec![("amenity", "dentist"), ("healthcare", "dentist")], "dent|odonto".into())
    } else if has(&["nutri"]) {
        (vec![("healthcare", "dietitian")], "nutri".into())
    } else if has(&["advog", "advocacia", "juridico"]) {
        (vec![("office", "lawyer")], "advog".into())
    } else if has(&["academia", "fitness", "crossfit", "musculacao"]) {
        (vec![("leisure", "fitness_centre")], "academia|fitness|crossfit".into())
    } else if has(&["barbearia", "barbear", "barber"]) {
        (vec![("shop", "hairdresser")], "barbear|barber".into())
    } else if has(&["salao", "beleza", "cabeleireiro", "estetica", "manicure"]) {
        (
            vec![("shop", "hairdresser"), ("shop", "beauty")],
            "salao|beleza|cabelo|estetica".into(),
        )
    } else if has(&["restaurante"]) {
        (vec![("amenity", "restaurant")], "restaurante".into())
    } else if has(&["pet", "veterin"]) {
        (vec![("shop", "pet"), ("amenity", "veterinary")], "pet|veterin".into())
    } else if has(&["imobiliaria", "imoveis"]) {
        (vec![("office", "estate_agent")], "imobili".into())
    } else if has(&["oficina", "mecanico", "mecanica"]) {
        (vec![("shop", "car_repair")], "oficina|mecan".into())
    } else if has(&["clinica"]) {
        (
            vec![("amenity", "clinic"), ("amenity", "doctors"), ("healthcare", "clinic")],
            "clinica".into(),
        )
    } else if has(&["farmacia", "drogaria"]) {
        (vec![("amenity", "pharmacy")], "farmacia|drogaria".into())
    } else if has(&["padaria"]) {
        (vec![("shop", "bakery")], "padaria".into())
    } else if has(&["mercado", "supermercado"]) {
        (vec![("shop", "supermarket"), ("shop", "convenience")], "mercado".into())
    } else if has(&["hotel", "pousada"]) {
        (vec![("tourism", "hotel"), ("tourism", "guest_house")], "hotel|pousada".into())
    } else {
        let rx = query
            .split_whitespace()
            .map(escape_rx)
            .filter(|w| w.len() >= 3)
            .collect::<Vec<_>>()
            .join("|");
        (vec![], if rx.is_empty() { ".+".into() } else { rx })
    }
}

fn escape_rx(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if ".+*?()[]{}^$|\\".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Monta Overpass QL: busca por tags + fallback por nome, num raio (m).
pub fn build_query(query: &str, lat: f64, lng: f64, radius_meters: f64, limit: usize) -> String {
    let (tags, name_rx) = niche_rule(query);
    let r = radius_meters.round() as i64;
    let mut parts = Vec::new();
    for (k, v) in &tags {
        for geom in ["node", "way", "relation"] {
            parts.push(format!(r#"{geom}["{k}"="{v}"](around:{r},{lat},{lng});"#));
        }
    }
    for geom in ["node", "way", "relation"] {
        parts.push(format!(r#"{geom}["name"~"{name_rx}",i](around:{r},{lat},{lng});"#));
    }
    format!(
        "[out:json][timeout:50];({});out center {limit};",
        parts.join("")
    )
}

#[derive(Debug, Deserialize, Default)]
struct OverpassResponse {
    #[serde(default)]
    elements: Vec<Element>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct Element {
    #[serde(rename = "type")]
    #[serde(default)]
    etype: String,
    #[serde(default)]
    id: i64,
    #[serde(default)]
    lat: Option<f64>,
    #[serde(default)]
    lon: Option<f64>,
    #[serde(default)]
    center: Option<Center>,
    #[serde(default)]
    tags: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Default)]
struct Center {
    #[serde(default)]
    lat: Option<f64>,
    #[serde(default)]
    lon: Option<f64>,
}

fn compose_address(tags: &HashMap<String, String>) -> Option<String> {
    let street = tags.get("addr:street");
    let number = tags.get("addr:housenumber");
    let suburb = tags.get("addr:suburb").or_else(|| tags.get("addr:district"));
    let city = tags.get("addr:city");
    let mut parts = Vec::new();
    match (street, number) {
        (Some(s), Some(n)) => parts.push(format!("{s}, {n}")),
        (Some(s), None) => parts.push(s.clone()),
        _ => {}
    }
    if let Some(s) = suburb {
        parts.push(s.clone());
    }
    if let Some(s) = city {
        parts.push(s.clone());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" - "))
    }
}

pub fn adapt_element(e: &Element) -> Option<DiscoveredPlace> {
    let name = e.tags.get("name").filter(|s| !s.trim().is_empty())?.clone();
    let (lat, lng) = match (e.lat, e.lon) {
        (Some(la), Some(lo)) => (Some(la), Some(lo)),
        _ => (e.center.as_ref()?.lat, e.center.as_ref()?.lon),
    };
    let category = e
        .tags
        .get("amenity")
        .or_else(|| e.tags.get("shop"))
        .or_else(|| e.tags.get("office"))
        .or_else(|| e.tags.get("healthcare"))
        .or_else(|| e.tags.get("leisure"))
        .or_else(|| e.tags.get("tourism"))
        .cloned();
    Some(DiscoveredPlace {
        external_id: format!("osm:{}/{}", e.etype, e.id),
        name,
        category,
        latitude: lat,
        longitude: lng,
        address: compose_address(&e.tags),
        provider: "osm".into(),
        phone: e
            .tags
            .get("phone")
            .or_else(|| e.tags.get("contact:phone"))
            .cloned(),
        website: e
            .tags
            .get("website")
            .or_else(|| e.tags.get("contact:website"))
            .cloned(),
        rating: None,
        review_count: None,
    })
}

pub struct OsmProvider {
    client: reqwest::Client,
    endpoint: String,
}

impl OsmProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .user_agent("LocalLeadProspector/0.1")
                .build()
                .expect("http client"),
            endpoint: OVERPASS_URL.into(),
        }
    }

    #[cfg(test)]
    fn test() -> Self {
        Self {
            client: reqwest::Client::builder().build().expect("http client"),
            endpoint: "http://localhost:9".into(),
        }
    }

    pub async fn search_at(
        &self,
        query: &str,
        lat: f64,
        lng: f64,
        radius_meters: f64,
    ) -> Result<Vec<DiscoveredPlace>, AppError> {
        let ql = build_query(query, lat, lng, radius_meters, 300);
        let resp = self
            .client
            .post(&self.endpoint)
            .form(&[("data", ql)])
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AppError::Provider("mapa demorou a responder — tente de novo".into())
                } else {
                    AppError::Network("mapa indisponível — verifique sua internet".into())
                }
            })?;
        let status = resp.status();
        if status.as_u16() == 429 || status.as_u16() == 504 {
            return Err(AppError::Provider(
                "mapa gratuito ocupado — espere alguns segundos e tente de novo".into(),
            ));
        }
        if !status.is_success() {
            return Err(AppError::Provider(format!("mapa retornou http {status}")));
        }
        let body: OverpassResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Parse(format!("resposta do mapa: {e}")))?;
        Ok(body.elements.iter().filter_map(adapt_element).collect())
    }
}

impl Default for OsmProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_tag_and_name_query() {
        let q = build_query("Dentista", -7.08, -41.46, 5000.0, 300);
        assert!(q.contains(r#"["amenity"="dentist"]"#));
        assert!(q.contains("around:5000,-7.08,-41.46"));
        assert!(q.contains("out center 300"));
    }

    #[test]
    fn falls_back_to_words_for_unknown_niche() {
        let q = build_query("Vidraçaria", 0.0, 0.0, 1000.0, 10);
        assert!(q.contains("vidra"));
        assert!(!q.contains("amenity"));
    }

    #[test]
    fn adapts_node_element() {
        let e = Element {
            etype: "node".into(),
            id: 123,
            lat: Some(-7.08),
            lon: Some(-41.46),
            center: None,
            tags: [
                ("name".into(), "Sorriso Legal".into()),
                ("amenity".into(), "dentist".into()),
                ("addr:street".into(), "Rua A".into()),
                ("addr:housenumber".into(), "10".into()),
                ("phone".into(), "+55".into()),
            ]
            .into_iter()
            .collect(),
        };
        let d = adapt_element(&e).unwrap();
        assert_eq!(d.external_id, "osm:node/123");
        assert_eq!(d.provider, "osm");
        assert_eq!(d.address.as_deref(), Some("Rua A, 10"));
    }

    #[test]
    fn skips_nameless_elements() {
        assert!(adapt_element(&Element::default()).is_none());
    }
}
