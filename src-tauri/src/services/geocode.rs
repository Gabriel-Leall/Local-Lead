use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct NominatimItem {
    lat: String,
    lon: String,
    #[serde(default)]
    display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CitySuggestion {
    pub display_name: String,
    pub lat: f64,
    pub lon: f64,
}

pub fn parse_suggestions(body: &str) -> Vec<CitySuggestion> {
    let items: Vec<NominatimItem> = serde_json::from_str(body).unwrap_or_default();
    items
        .into_iter()
        .filter_map(|it| {
            Some(CitySuggestion {
                display_name: it.display_name?,
                lat: it.lat.parse().ok()?,
                lon: it.lon.parse().ok()?,
            })
        })
        .take(5)
        .collect()
}

pub fn parse_nominatim(body: &str) -> Option<(f64, f64)> {
    let items: Vec<NominatimItem> = serde_json::from_str(body).ok()?;
    let first = items.into_iter().next()?;
    Some((first.lat.parse().ok()?, first.lon.parse().ok()?))
}

fn nominatim_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("LocalLeadProspector/0.1")
        .build()
        .map_err(|e| AppError::Network(e.to_string()))
}

pub async fn geocode_city(city: &str) -> Result<(f64, f64), AppError> {
    if city.trim().is_empty() {
        return Err(AppError::InvalidRequest("informe a cidade".into()));
    }
    let client = nominatim_client()?;
    let resp = client
        .get("https://nominatim.openstreetmap.org/search")
        .query(&[("q", city), ("format", "json".into()), ("limit", "1".into())])
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Provider(format!("geocodificação http {}", resp.status())));
    }
    let text = resp.text().await.map_err(|e| AppError::Parse(e.to_string()))?;
    parse_nominatim(&text).ok_or_else(|| AppError::InvalidRequest(format!("cidade não encontrada: {city}")))
}

pub async fn autocomplete_city(query: &str) -> Result<Vec<CitySuggestion>, AppError> {
    if query.trim().len() < 3 {
        return Ok(vec![]);
    }
    let client = nominatim_client()?;
    let resp = client
        .get("https://nominatim.openstreetmap.org/search")
        .query(&[
            ("q", query),
            ("format", "json".into()),
            ("addressdetails", "1".into()),
            ("limit", "5".into()),
        ])
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Provider("falha no autocomplete".into()));
    }
    let text = resp.text().await.map_err(|e| AppError::Parse(e.to_string()))?;
    Ok(parse_suggestions(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nominatim_response() {
        let body = r#"[{"lat":"25.7616798","lon":"-80.1917902"}]"#;
        let (lat, _lng) = parse_nominatim(body).unwrap();
        assert!((lat - 25.76).abs() < 0.01);
    }

    #[test]
    fn returns_none_on_empty() {
        assert!(parse_nominatim("[]").is_none());
    }

    #[test]
    fn parses_suggestions() {
        let body = r#"[{"lat":"-7.23","lon":"-41.46","display_name":"Picos, Piauí, Brasil"}]"#;
        let s = parse_suggestions(body);
        assert_eq!(s.len(), 1);
        assert!(s[0].display_name.contains("Picos"));
    }

    #[test]
    fn hints_disabled_api() {
        let h = crate::error::google_error_hint(r#"{"error":{"code":403,"message":"Places API (New) has not been used in project","status":"PERMISSION_DENIED"}}"#);
        assert!(h.contains("não está ativada"));
    }
}
