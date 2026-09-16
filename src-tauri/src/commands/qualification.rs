use crate::database::repositories::{
    bulk_update_status, delete_saved_filter, get_config, list_leads_filtered, list_saved_filters,
    save_filter, set_config, LeadFilters, SavedFilter,
};
use crate::database::DbState;
use crate::domain::Lead;
use crate::error::AppError;
use crate::services::lead_scoring::{rescore_all, ScoreConfig};
use serde::Serialize;
use tauri::State;

const SCORE_KEY: &str = "score_config";

fn lock(db: &DbState) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, AppError> {
    db.0.lock().map_err(|e| AppError::Database(e.to_string()))
}

fn load_config(conn: &rusqlite::Connection) -> ScoreConfig {
    let raw: Option<String> = get_config(conn, SCORE_KEY).ok().flatten();
    raw.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

#[tauri::command]
pub fn get_score_config(db: State<'_, DbState>) -> Result<ScoreConfig, AppError> {
    let conn = lock(&db)?;
    Ok(load_config(&conn))
}

#[tauri::command]
pub fn update_score_config(
    db: State<'_, DbState>,
    config: ScoreConfig,
) -> Result<ScoreConfig, AppError> {
    let conn = lock(&db)?;
    let json = serde_json::to_string(&config).map_err(|e| AppError::InvalidRequest(e.to_string()))?;
    set_config(&conn, SCORE_KEY, &json)?;
    Ok(config)
}

#[derive(Serialize)]
pub struct RescoreResponse {
    pub rescored: i64,
}

#[tauri::command]
pub fn rescore_leads(db: State<'_, DbState>) -> Result<RescoreResponse, AppError> {
    let conn = lock(&db)?;
    let cfg = load_config(&conn);
    let n = rescore_all(&conn, &cfg)?;
    Ok(RescoreResponse { rescored: n })
}

#[tauri::command]
pub fn get_qualification_queue(
    db: State<'_, DbState>,
    limit: Option<i64>,
) -> Result<Vec<Lead>, AppError> {
    let conn = lock(&db)?;
    Ok(list_leads_filtered(
        &conn,
        &LeadFilters {
            name: None,
            category: None,
            status: Some("new"),
            has_website: None,
            has_phone: None,
            has_email: None,
            has_instagram: None,
            website_status: None,
            min_rating: None,
            min_reviews: None,
            min_score: None,
            order_by_score: true,
        },
        limit.unwrap_or(50),
    )?)
}

#[tauri::command]
pub fn bulk_set_status(
    db: State<'_, DbState>,
    lead_ids: Vec<i64>,
    status: String,
) -> Result<usize, AppError> {
    if !crate::domain::is_valid_status(&status) {
        return Err(AppError::InvalidRequest("invalid status".into()));
    }
    if lead_ids.is_empty() || lead_ids.len() > 500 {
        return Err(AppError::InvalidRequest("select 1-500 leads".into()));
    }
    let conn = lock(&db)?;
    let mut n = 0;
    for id in &lead_ids {
        crate::database::repositories::change_lead_status(&conn, *id, &status, None)?;
        n += 1;
    }
    Ok(n)
}

#[tauri::command]
pub fn list_filters(db: State<'_, DbState>) -> Result<Vec<SavedFilter>, AppError> {
    let conn = lock(&db)?;
    Ok(list_saved_filters(&conn)?)
}

#[tauri::command]
pub fn save_named_filter(
    db: State<'_, DbState>,
    name: String,
    filters_json: String,
) -> Result<i64, AppError> {
    if name.trim().is_empty() || name.len() > 60 {
        return Err(AppError::InvalidRequest("name required (max 60)".into()));
    }
    if filters_json.len() > 5000 {
        return Err(AppError::InvalidRequest("filter too large".into()));
    }
    serde_json::from_str::<serde_json::Value>(&filters_json)
        .map_err(|_| AppError::InvalidRequest("invalid filters_json".into()))?;
    let conn = lock(&db)?;
    Ok(save_filter(&conn, name.trim(), &filters_json)?)
}

#[tauri::command]
pub fn delete_filter(db: State<'_, DbState>, id: i64) -> Result<(), AppError> {
    let conn = lock(&db)?;
    delete_saved_filter(&conn, id)?;
    Ok(())
}
