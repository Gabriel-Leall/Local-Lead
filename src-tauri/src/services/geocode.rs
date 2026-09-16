use crate::error::AppError;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct NominatimItem {
    lat: String,
    lon: String,
}

pub fn parse_nominatim(body: &str) -> Option<(f64, f64)> {
    let items: Vec<NominatimItem> = serde_json::from_str(body).ok()?;
    let first = items.into_iter().next()?;
    Some((first.lat.parse().ok()?, first.lon.parse().ok()?))
}

pub async fn geocode_city(city: &str) -> Result<(f64, f64), AppError> {
    if city.trim().is_empty() {
        return Err(AppError::InvalidRequest("city is required".into()));
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("LocalLeadProspector/0.1")
        .build()
        .map_err(|e| AppError::Network(e.to_string()))?;
    let resp = client
        .get("https://nominatim.openstreetmap.org/search")
        .query(&[("q", city), ("format", "json".into()), ("limit", "1".into())])
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Provider(format!("geocode http {}", resp.status())));
    }
    let text = resp.text().await.map_err(|e| AppError::Parse(e.to_string()))?;
    parse_nominatim(&text).ok_or_else(|| AppError::InvalidRequest(format!("city not found: {city}")))
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
}
