use super::{DiscoveryProvider, RegionSearchRequest, SearchRequest};
use crate::domain::DiscoveredPlace;
use crate::error::AppError;
use serde::{Deserialize, Serialize};

const TEXT_SEARCH_URL: &str = "https://places.googleapis.com/v1/places:searchText";
const FIELD_MASK: &str = "places.id,places.displayName,places.primaryType,places.location,places.formattedAddress,places.rating,places.userRatingCount,places.nationalPhoneNumber,places.websiteUri";
const DETAILS_MASK: &str = "id,displayName,primaryType,location,formattedAddress,rating,userRatingCount,nationalPhoneNumber,internationalPhoneNumber,websiteUri";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextSearchBody {
    text_query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    location_bias: Option<LocationBias>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_size: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocationBias {
    circle: Circle,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Circle {
    center: Center,
    radius: f64,
}

#[derive(Debug, Serialize)]
struct Center {
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct TextSearchResponse {
    #[serde(default)]
    places: Vec<GooglePlace>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GooglePlace {
    pub id: Option<String>,
    #[serde(default)]
    pub display_name: Option<DisplayName>,
    pub primary_type: Option<String>,
    pub location: Option<LatLng>,
    pub formatted_address: Option<String>,
    pub rating: Option<f64>,
    pub user_rating_count: Option<i64>,
    pub national_phone_number: Option<String>,
    pub website_uri: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct DisplayName {
    pub text: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct LatLng {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

pub fn map_place(p: &GooglePlace) -> Option<DiscoveredPlace> {
    let external_id = p.id.clone()?;
    let name = p.display_name.clone().and_then(|d| d.text)?;
    Some(DiscoveredPlace {
        external_id,
        name,
        category: p.primary_type.clone(),
        latitude: p.location.clone().and_then(|l| l.latitude),
        longitude: p.location.clone().and_then(|l| l.longitude),
        address: p.formatted_address.clone(),
        provider: "google_places".into(),
        phone: p.national_phone_number.clone(),
        website: p.website_uri.clone(),
        rating: p.rating,
        review_count: p.user_rating_count,
    })
}

pub struct GooglePlacesProvider {
    client: reqwest::Client,
}

impl GooglePlacesProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .expect("http client"),
        }
    }
}

impl Default for GooglePlacesProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DiscoveryProvider for GooglePlacesProvider {
    async fn search(&self, req: &SearchRequest) -> Result<Vec<DiscoveredPlace>, AppError> {
        let text_query = format!("{} {}", req.query.trim(), req.city.trim());
        self.post_text_search(&req.api_key, text_query, None).await
    }

    async fn search_region(&self, req: &RegionSearchRequest) -> Result<Vec<DiscoveredPlace>, AppError> {
        let bias = LocationBias {
            circle: Circle {
                center: Center { latitude: req.center_lat, longitude: req.center_lng },
                radius: req.radius_meters.min(50_000.0).max(200.0),
            },
        };
        self.post_text_search(&req.api_key, req.query.clone(), Some(bias)).await
    }
}

#[derive(Debug, Clone, Default)]
pub struct PlaceDetails {
    pub phone: Option<String>,
    pub website: Option<String>,
    pub rating: Option<f64>,
    pub review_count: Option<i64>,
    pub address: Option<String>,
    pub category: Option<String>,
}

pub fn map_details(p: &GooglePlace) -> PlaceDetails {
    PlaceDetails {
        phone: p.national_phone_number.clone(),
        website: p.website_uri.clone(),
        rating: p.rating,
        review_count: p.user_rating_count,
        address: p.formatted_address.clone(),
        category: p.primary_type.clone(),
    }
}

impl GooglePlacesProvider {
    pub async fn fetch_details(&self, place_id: &str, api_key: &str) -> Result<PlaceDetails, AppError> {
        if api_key.trim().is_empty() {
            return Err(AppError::MissingApiKey);
        }
        let short_id = place_id.strip_prefix("places/").unwrap_or(place_id);
        let url = format!("https://places.googleapis.com/v1/places/{short_id}");
        let resp = self
            .client
            .get(&url)
            .header("X-Goog-Api-Key", api_key)
            .header("X-Goog-FieldMask", DETAILS_MASK)
            .send()
            .await?;
        let status = resp.status();
        if status.as_u16() == 400 {
            return Err(AppError::InvalidRequest("bad details request".into()));
        }
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(AppError::InvalidApiKey);
        }
        if status.as_u16() == 429 {
            return Err(AppError::RateLimit);
        }
        if status.as_u16() == 404 {
            return Err(AppError::InvalidRequest("place not found".into()));
        }
        if !status.is_success() {
            return Err(AppError::Provider(format!("details http {status}")));
        }
        let place: GooglePlace = resp.json().await.map_err(|e| AppError::Parse(e.to_string()))?;
        Ok(map_details(&place))
    }

    async fn post_text_search(
        &self,
        api_key: &str,
        text_query: String,
        location_bias: Option<LocationBias>,
    ) -> Result<Vec<DiscoveredPlace>, AppError> {
        let resp = self
            .client
            .post(TEXT_SEARCH_URL)
            .header("Content-Type", "application/json")
            .header("X-Goog-Api-Key", api_key)
            .header("X-Goog-FieldMask", FIELD_MASK)
            .json(&TextSearchBody { text_query, location_bias, page_size: Some(20) })
            .send()
            .await?;

        let status = resp.status();
        if status.as_u16() == 400 {
            return Err(AppError::InvalidRequest("bad request — check query".into()));
        }
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(AppError::InvalidApiKey);
        }
        if status.as_u16() == 429 {
            return Err(AppError::RateLimit);
        }
        if !status.is_success() {
            return Err(AppError::Provider(format!("http {status}")));
        }

        let body: TextSearchResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Parse(e.to_string()))?;

        Ok(body.places.iter().filter_map(map_place).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_minimal_place() {
        let p = GooglePlace {
            id: Some("places/abc".into()),
            display_name: Some(DisplayName {
                text: Some("Miami Dental".into()),
            }),
            primary_type: Some("dentist".into()),
            location: Some(LatLng {
                latitude: Some(25.7),
                longitude: Some(-80.3),
            }),
            formatted_address: Some("Miami, FL".into()),
            ..Default::default()
        };
        let m = map_place(&p).unwrap();
        assert_eq!(m.external_id, "places/abc");
        assert_eq!(m.provider, "google_places");
        assert_eq!(m.latitude, Some(25.7));
    }

    #[test]
    fn skips_place_without_id_or_name() {
        let p = GooglePlace::default();
        assert!(map_place(&p).is_none());
    }

    #[test]
    fn maps_details_fields() {
        let p = GooglePlace {
            national_phone_number: Some("+1 305-000-0000".into()),
            website_uri: Some("https://example.com".into()),
            rating: Some(4.8),
            user_rating_count: Some(281),
            formatted_address: Some("Miami, FL".into()),
            primary_type: Some("dentist".into()),
            ..Default::default()
        };
        let d = map_details(&p);
        assert_eq!(d.rating, Some(4.8));
        assert_eq!(d.review_count, Some(281));
        assert_eq!(d.website.as_deref(), Some("https://example.com"));
    }
}
