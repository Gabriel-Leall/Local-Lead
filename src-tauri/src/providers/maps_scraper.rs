use crate::domain::DiscoveredPlace;
use serde::{Deserialize, Serialize};

/// Subset of gosom/google-maps-scraper JSON output.
/// Unknown fields are ignored so exports keep working across versions.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScraperRecord {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub rating: Option<f64>,
    #[serde(default)]
    pub reviews: Option<i64>,
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    #[serde(default)]
    pub google_id: Option<String>,
    #[serde(default)]
    pub place_id: Option<String>,
}

pub fn adapt_record(r: &ScraperRecord, idx: usize) -> Option<DiscoveredPlace> {
    let name = r.name.clone().filter(|s| !s.trim().is_empty())?;
    let external_id = r
        .place_id
        .clone()
        .or_else(|| r.google_id.clone())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("scraper-{idx}-{}", name.to_lowercase().replace(' ', "-")));
    Some(DiscoveredPlace {
        external_id,
        name,
        category: r.category.clone(),
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

pub fn parse_records(json: &str) -> Result<Vec<ScraperRecord>, String> {
    if let Ok(arr) = serde_json::from_str::<Vec<ScraperRecord>>(json) {
        return Ok(arr);
    }
    if let Ok(single) = serde_json::from_str::<ScraperRecord>(json) {
        return Ok(vec![single]);
    }
    Err("invalid scraper JSON — expected array of businesses".into())
}

pub fn check_binary() -> Result<String, String> {
    let out = std::process::Command::new("google-maps-scraper")
        .arg("--help")
        .output();
    match out {
        Ok(o) if o.status.success() => Ok("google-maps-scraper found in PATH".into()),
        Ok(o) => Err(format!("binary returned {}", o.status)),
        Err(_) => Err("google-maps-scraper not found in PATH — use JSON import instead".into()),
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
    fn parses_array_json() {
        let json = r#"[{"name":"A"},{"name":"B","rating":4.5}]"#;
        assert_eq!(parse_records(json).unwrap().len(), 2);
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_records("not json").is_err());
    }
}
