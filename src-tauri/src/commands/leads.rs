use crate::database::repositories::{change_lead_status, list_leads};
use crate::database::DbState;
use crate::domain::{is_valid_status, Lead};
use crate::error::AppError;
use tauri::State;

#[tauri::command]
pub fn get_leads(
    db: State<'_, DbState>,
    name_filter: Option<String>,
    category_filter: Option<String>,
    status_filter: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<Lead>, AppError> {
    let conn = db.0.lock().map_err(|e| AppError::Database(e.to_string()))?;
    Ok(list_leads(
        &conn,
        name_filter.as_deref(),
        category_filter.as_deref(),
        status_filter.as_deref(),
        limit.unwrap_or(200),
    )?)
}

#[tauri::command]
pub fn update_lead_status(
    db: State<'_, DbState>,
    id: i64,
    status: String,
) -> Result<(), AppError> {
    if !is_valid_status(&status) {
        return Err(AppError::InvalidRequest("invalid status".into()));
    }
    let conn = db.0.lock().map_err(|e| AppError::Database(e.to_string()))?;
    change_lead_status(&conn, id, &status, None)?;
    Ok(())
}
