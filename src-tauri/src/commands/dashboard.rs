use crate::database::DbState;
use crate::error::AppError;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct DashboardStats {
    pub total_leads: i64,
    pub new_leads: i64,
    pub searches_completed: i64,
    pub qualified: i64,
    pub contacted: i64,
    pub replied: i64,
    pub won: i64,
    pub without_website: i64,
    pub with_email: i64,
}

fn count(conn: &rusqlite::Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap_or(0)
}

#[tauri::command]
pub fn get_dashboard_stats(db: State<'_, DbState>) -> Result<DashboardStats, AppError> {
    let conn = db.0.lock().map_err(|e| AppError::Database(e.to_string()))?;
    Ok(DashboardStats {
        total_leads: count(&conn, "SELECT COUNT(*) FROM leads"),
        new_leads: count(&conn, "SELECT COUNT(*) FROM leads WHERE lead_status='new'"),
        searches_completed: count(&conn, "SELECT COUNT(*) FROM search_jobs WHERE status='completed'"),
        qualified: count(&conn, "SELECT COUNT(*) FROM leads WHERE lead_status='qualified'"),
        contacted: count(&conn, "SELECT COUNT(*) FROM leads WHERE lead_status IN ('contacted','message_sent')"),
        replied: count(&conn, "SELECT COUNT(*) FROM leads WHERE lead_status IN ('replied','responded','interested','meeting')"),
        won: count(&conn, "SELECT COUNT(*) FROM leads WHERE lead_status='won'"),
        without_website: count(&conn, "SELECT COUNT(*) FROM leads WHERE website IS NULL OR website=''"),
        with_email: count(&conn, "SELECT COUNT(*) FROM leads WHERE email IS NOT NULL AND email!=''"),
    })
}
