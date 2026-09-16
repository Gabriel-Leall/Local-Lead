use crate::database::repositories::{add_lead_note, change_lead_status, list_history, list_notes, LeadNote, StatusEvent};
use crate::database::DbState;
use crate::error::AppError;
use serde::Serialize;
use tauri::State;

fn lock(db: &DbState) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, AppError> {
    db.0.lock().map_err(|e| AppError::Database(e.to_string()))
}

#[derive(Serialize)]
pub struct Activity {
    pub history: Vec<StatusEvent>,
    pub notes: Vec<LeadNote>,
}

#[tauri::command]
pub fn get_activity(db: State<'_, DbState>, lead_id: i64) -> Result<Activity, AppError> {
    let conn = lock(&db)?;
    Ok(Activity {
        history: list_history(&conn, lead_id)?,
        notes: list_notes(&conn, lead_id)?,
    })
}

#[tauri::command]
pub fn add_note(
    db: State<'_, DbState>,
    lead_id: i64,
    note: String,
    follow_up_at: Option<String>,
) -> Result<i64, AppError> {
    if note.trim().is_empty() || note.len() > 2000 {
        return Err(AppError::InvalidRequest("note 1-2000 chars".into()));
    }
    let conn = lock(&db)?;
    Ok(add_lead_note(&conn, lead_id, note.trim(), follow_up_at.as_deref())?)
}

#[tauri::command]
pub fn set_pipeline_status(
    db: State<'_, DbState>,
    lead_id: i64,
    status: String,
    note: Option<String>,
) -> Result<(), AppError> {
    if !crate::domain::is_valid_status(&status) {
        return Err(AppError::InvalidRequest("invalid status".into()));
    }
    let conn = lock(&db)?;
    change_lead_status(&conn, lead_id, &status, note.as_deref())?;
    Ok(())
}
