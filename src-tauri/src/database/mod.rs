pub mod repositories;

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct DbState(pub Mutex<Connection>);

pub fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("prospector.db"))
}

pub fn ensure_columns(conn: &Connection) {
    let _ = conn.execute("ALTER TABLE search_jobs ADD COLUMN center_lat REAL", []);
    let _ = conn.execute("ALTER TABLE search_jobs ADD COLUMN center_lng REAL", []);
    let _ = conn.execute("ALTER TABLE search_jobs ADD COLUMN strategy TEXT NOT NULL DEFAULT 'single'", []);
    let _ = conn.execute("ALTER TABLE discoveries ADD COLUMN cell_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE leads ADD COLUMN score INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE leads ADD COLUMN score_reasons TEXT", []);
    let _ = conn.execute("ALTER TABLE leads ADD COLUMN email TEXT", []);
    let _ = conn.execute("ALTER TABLE leads ADD COLUMN instagram TEXT", []);
    let _ = conn.execute("ALTER TABLE leads ADD COLUMN facebook TEXT", []);
    let _ = conn.execute("ALTER TABLE leads ADD COLUMN whatsapp TEXT", []);
    let _ = conn.execute("ALTER TABLE leads ADD COLUMN website_status TEXT", []);
    let _ = conn.execute("ALTER TABLE leads ADD COLUMN follow_up_at TEXT", []);
}

pub fn mark_interrupted(conn: &Connection) {
    let _ = conn.execute(
        "UPDATE search_jobs SET status='interrupted', completed_at=NULL WHERE status IN ('running','pending')",
        [],
    );
    let _ = conn.execute(
        "UPDATE search_cells SET status='interrupted' WHERE status IN ('running','pending')",
        [],
    );
}

pub fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode=WAL;
        CREATE TABLE IF NOT EXISTS leads (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            canonical_name TEXT NOT NULL,
            category TEXT,
            address TEXT,
            latitude REAL,
            longitude REAL,
            phone TEXT,
            website TEXT,
            rating REAL,
            review_count INTEGER,
            email TEXT,
            instagram TEXT,
            facebook TEXT,
            whatsapp TEXT,
            website_status TEXT,
            lead_status TEXT NOT NULL DEFAULT 'new',
            score INTEGER NOT NULL DEFAULT 0,
            score_reasons TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS external_places (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            lead_id INTEGER NOT NULL,
            provider TEXT NOT NULL,
            external_place_id TEXT,
            raw_name TEXT,
            raw_address TEXT,
            raw_payload_json TEXT,
            discovered_at TEXT NOT NULL,
            UNIQUE(provider, external_place_id),
            FOREIGN KEY(lead_id) REFERENCES leads(id)
        );
        CREATE TABLE IF NOT EXISTS search_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT,
            query TEXT NOT NULL,
            city TEXT NOT NULL,
            radius_meters REAL,
            center_lat REAL,
            center_lng REAL,
            strategy TEXT NOT NULL DEFAULT 'single',
            status TEXT NOT NULL DEFAULT 'pending',
            result_count INTEGER NOT NULL DEFAULT 0,
            new_count INTEGER NOT NULL DEFAULT 0,
            error TEXT,
            created_at TEXT NOT NULL,
            completed_at TEXT
        );
        CREATE TABLE IF NOT EXISTS discoveries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            lead_id INTEGER NOT NULL,
            job_id INTEGER NOT NULL,
            cell_id INTEGER,
            provider TEXT,
            query TEXT,
            discovered_at TEXT NOT NULL,
            FOREIGN KEY(lead_id) REFERENCES leads(id),
            FOREIGN KEY(job_id) REFERENCES search_jobs(id)
        );
        CREATE TABLE IF NOT EXISTS search_cells (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id INTEGER NOT NULL,
            parent_id INTEGER,
            north REAL NOT NULL,
            south REAL NOT NULL,
            east REAL NOT NULL,
            west REAL NOT NULL,
            depth INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            raw_result_count INTEGER NOT NULL DEFAULT 0,
            new_unique_count INTEGER NOT NULL DEFAULT 0,
            query TEXT NOT NULL,
            provider TEXT NOT NULL DEFAULT 'google_places',
            created_at TEXT NOT NULL,
            completed_at TEXT,
            FOREIGN KEY(job_id) REFERENCES search_jobs(id)
        );
        CREATE INDEX IF NOT EXISTS idx_cells_job ON search_cells(job_id);
        CREATE INDEX IF NOT EXISTS idx_cells_status ON search_cells(status);
        CREATE TABLE IF NOT EXISTS enrichment_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            lead_id INTEGER NOT NULL,
            enrichment_type TEXT NOT NULL DEFAULT 'place_details',
            status TEXT NOT NULL DEFAULT 'pending',
            attempts INTEGER NOT NULL DEFAULT 0,
            error TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(lead_id) REFERENCES leads(id)
        );
        CREATE INDEX IF NOT EXISTS idx_enrich_lead ON enrichment_jobs(lead_id);
        CREATE TABLE IF NOT EXISTS saved_filters (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            filters_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS app_config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS outreach_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            lead_id INTEGER NOT NULL,
            channel TEXT NOT NULL,
            subject TEXT,
            message TEXT NOT NULL,
            template_id TEXT,
            provider TEXT NOT NULL DEFAULT 'template',
            status TEXT NOT NULL DEFAULT 'generated',
            created_at TEXT NOT NULL,
            approved_at TEXT,
            sent_at TEXT,
            FOREIGN KEY(lead_id) REFERENCES leads(id)
        );
        CREATE INDEX IF NOT EXISTS idx_msg_lead ON outreach_messages(lead_id);
        CREATE TABLE IF NOT EXISTS lead_status_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            lead_id INTEGER NOT NULL,
            from_status TEXT,
            to_status TEXT NOT NULL,
            note TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY(lead_id) REFERENCES leads(id)
        );
        CREATE TABLE IF NOT EXISTS lead_notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            lead_id INTEGER NOT NULL,
            note TEXT NOT NULL,
            follow_up_at TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY(lead_id) REFERENCES leads(id)
        );
        CREATE INDEX IF NOT EXISTS idx_hist_lead ON lead_status_history(lead_id);
        CREATE INDEX IF NOT EXISTS idx_notes_lead ON lead_notes(lead_id);
        CREATE INDEX IF NOT EXISTS idx_leads_name ON leads(canonical_name);
        CREATE INDEX IF NOT EXISTS idx_leads_status ON leads(lead_status);
        CREATE INDEX IF NOT EXISTS idx_leads_score ON leads(score DESC);
        "#,
    )?;
    ensure_columns(&conn);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_run_in_memory() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM leads", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
