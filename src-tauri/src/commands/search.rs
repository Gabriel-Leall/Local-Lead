use crate::database::DbState;
use crate::error::AppError;
use crate::services::search_service::{run_search, RunSearchInput};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct SearchResult {
    pub job_id: i64,
    pub result_count: i64,
    pub new_count: i64,
}

#[derive(Serialize)]
pub struct KeyTest {
    pub ok: bool,
    pub found: usize,
}

#[tauri::command]
pub async fn test_places_key(api_key: String) -> Result<KeyTest, AppError> {
    use crate::providers::{DiscoveryProvider, GooglePlacesProvider, RegionSearchRequest};
    if api_key.trim().is_empty() {
        return Err(AppError::MissingApiKey);
    }
    let provider = GooglePlacesProvider::new();
    let places = provider
        .search_region(&RegionSearchRequest {
            query: "dentista".into(),
            api_key,
            center_lat: -23.5558,
            center_lng: -46.6396,
            radius_meters: 5000.0,
        })
        .await?;
    Ok(KeyTest { ok: true, found: places.len() })
}

#[tauri::command]
pub async fn autocomplete_city_cmd(
    query: String,
) -> Result<Vec<crate::services::geocode::CitySuggestion>, AppError> {
    crate::services::geocode::autocomplete_city(&query).await
}

#[tauri::command]
pub async fn search_osm_cmd(
    db: State<'_, DbState>,
    query: String,
    city: String,
    radius_km: f64,
) -> Result<SearchResult, AppError> {
    let (job_id, result_count, new_count) =
        crate::services::osm_search::run_osm_search(&db, query, city, radius_km * 1000.0).await?;
    Ok(SearchResult { job_id, result_count, new_count })
}

#[tauri::command]
pub async fn search_leads(
    db: State<'_, DbState>,
    query: String,
    city: String,
    radius_km: Option<f64>,
    api_key: String,
) -> Result<SearchResult, AppError> {
    let radius_meters = radius_km.map(|km| km * 1000.0);
    let (job_id, result_count, new_count) = run_search(
        &db,
        RunSearchInput {
            query,
            city,
            radius_meters,
            api_key,
        },
    )
    .await?;
    Ok(SearchResult {
        job_id,
        result_count,
        new_count,
    })
}
