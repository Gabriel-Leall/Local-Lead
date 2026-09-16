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
