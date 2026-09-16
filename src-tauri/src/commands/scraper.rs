use crate::database::repositories::{create_job, finish_job, provider_counts, upsert_scraper_place, ProviderCount};
use crate::database::DbState;
use crate::error::AppError;
use crate::providers::maps_scraper::{adapt_record, check_binary, parse_records};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct BinaryStatus {
    pub available: bool,
    pub message: String,
}

#[tauri::command]
pub fn check_scraper_binary() -> BinaryStatus {
    match check_binary() {
        Ok(m) => BinaryStatus { available: true, message: m },
        Err(m) => BinaryStatus { available: false, message: m },
    }
}

#[derive(Serialize)]
pub struct ImportResponse {
    pub job_id: i64,
    pub imported: i64,
    pub merged: i64,
    pub skipped: i64,
}

#[derive(Serialize)]
pub struct StartResponse {
    pub job_id: i64,
}

#[tauri::command]
pub async fn start_scraper_search_cmd(
    db: State<'_, DbState>,
    jobs: State<'_, crate::services::scraper_runner::ScraperJobs>,
    query: String,
    city: String,
    radius_km: f64,
    with_email: Option<bool>,
) -> Result<StartResponse, AppError> {
    let job_id = crate::services::scraper_runner::start_scraper_search(
        &db,
        &jobs,
        query,
        city,
        radius_km * 1000.0,
        with_email.unwrap_or(false),
    )
    .await?;
    Ok(StartResponse { job_id })
}

#[tauri::command]
pub async fn poll_scraper_job_cmd(
    db: State<'_, DbState>,
    jobs: State<'_, crate::services::scraper_runner::ScraperJobs>,
    job_id: i64,
) -> Result<crate::services::scraper_runner::PollResult, AppError> {
    crate::services::scraper_runner::poll_scraper_job(&db, &jobs, job_id).await
}

#[tauri::command]
pub async fn cancel_scraper_search_cmd(
    db: State<'_, DbState>,
    jobs: State<'_, crate::services::scraper_runner::ScraperJobs>,
    job_id: i64,
) -> Result<(), AppError> {
    crate::services::scraper_runner::cancel_scraper_job(&db, &jobs, job_id).await
}

#[tauri::command]
pub fn import_scraper_json(
    db: State<'_, DbState>,
    query: String,
    city: String,
    results_json: String,
) -> Result<ImportResponse, AppError> {
    if query.trim().is_empty() {
        return Err(AppError::InvalidRequest("query is required".into()));
    }
    if results_json.len() > 2_000_000 {
        return Err(AppError::InvalidRequest("file too large (max 2MB)".into()));
    }
    let records = parse_records(&results_json).map_err(AppError::InvalidRequest)?;
    if records.is_empty() {
        return Err(AppError::InvalidRequest("no businesses in file".into()));
    }
    if records.len() > 1000 {
        return Err(AppError::InvalidRequest("max 1000 businesses per import".into()));
    }
    let conn = db.0.lock().map_err(|e| AppError::Database(e.to_string()))?;
    let job_id = create_job(&conn, &format!("{query} (scraper)"), &city, None)?;
    conn.execute(
        "UPDATE search_jobs SET strategy='scraper' WHERE id=?1",
        rusqlite::params![job_id],
    )?;
    let mut imported = 0i64;
    let mut merged = 0i64;
    let mut skipped = 0i64;
    for (idx, rec) in records.iter().enumerate() {
        let Some(place) = adapt_record(rec, idx) else {
            skipped += 1;
            continue;
        };
        match upsert_scraper_place(&conn, job_id, None, &place) {
            Ok((_, is_new)) => {
                if is_new {
                    imported += 1;
                } else {
                    merged += 1;
                }
            }
            Err(_) => skipped += 1,
        }
    }
    finish_job(&conn, job_id, "completed", imported + merged, imported, None)?;
    crate::services::lead_scoring::rescore_with_saved_config(&conn);
    Ok(ImportResponse { job_id, imported, merged, skipped })
}

#[tauri::command]
pub fn get_provider_counts(
    db: State<'_, DbState>,
    job_id: i64,
) -> Result<Vec<ProviderCount>, AppError> {
    let conn = db.0.lock().map_err(|e| AppError::Database(e.to_string()))?;
    Ok(provider_counts(&conn, job_id)?)
}
