use crate::database::repositories::{list_cells, list_jobs, set_job_status, JobSummary, SearchCell};
use crate::database::DbState;
use crate::error::AppError;
use crate::services::adaptive::{resume_adaptive_search, start_adaptive_search, AdaptiveInput};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct StartAdaptiveResult {
    pub job_id: i64,
}

#[tauri::command]
pub async fn start_adaptive_search_cmd(
    db: State<'_, DbState>,
    query: String,
    city: String,
    radius_km: f64,
    api_key: String,
) -> Result<StartAdaptiveResult, AppError> {
    let job_id = start_adaptive_search(
        &db,
        AdaptiveInput {
            query,
            city,
            radius_meters: radius_km * 1000.0,
            api_key,
        },
    )
    .await?;
    Ok(StartAdaptiveResult { job_id })
}

#[tauri::command]
pub fn get_search_jobs(db: State<'_, DbState>) -> Result<Vec<JobSummary>, AppError> {
    let conn = db.0.lock().map_err(|e| AppError::Database(e.to_string()))?;
    Ok(list_jobs(&conn)?)
}

#[tauri::command]
pub fn get_search_cells(db: State<'_, DbState>, job_id: i64) -> Result<Vec<SearchCell>, AppError> {
    let conn = db.0.lock().map_err(|e| AppError::Database(e.to_string()))?;
    Ok(list_cells(&conn, job_id)?)
}

#[tauri::command]
pub fn pause_search_job(db: State<'_, DbState>, job_id: i64) -> Result<(), AppError> {
    let conn = db.0.lock().map_err(|e| AppError::Database(e.to_string()))?;
    set_job_status(&conn, job_id, "paused", None)?;
    Ok(())
}

#[tauri::command]
pub async fn resume_search_job(
    db: State<'_, DbState>,
    job_id: i64,
    api_key: String,
) -> Result<(), AppError> {
    resume_adaptive_search(&db, job_id, &api_key).await
}

#[tauri::command]
pub fn cancel_search_job(db: State<'_, DbState>, job_id: i64) -> Result<(), AppError> {
    let conn = db.0.lock().map_err(|e| AppError::Database(e.to_string()))?;
    set_job_status(&conn, job_id, "cancelled", None)?;
    Ok(())
}
