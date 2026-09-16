use crate::database::repositories::{
    aggregate_job_counts, create_adaptive_job, create_cell, get_job_status, list_cells, mark_cell,
    pop_pending_cell, set_job_status, upsert_lead_in_cell,
};
use crate::database::DbState;
use crate::discovery::{should_split, SplitDecision, SplitPolicy};
use crate::domain::GeoRegion;
use crate::error::AppError;
use crate::providers::{DiscoveryProvider, GooglePlacesProvider, RegionSearchRequest};
use crate::services::geocode::geocode_city;

pub struct AdaptiveInput {
    pub query: String,
    pub city: String,
    pub radius_meters: f64,
    pub api_key: String,
}

async fn sleep_ms(ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

fn lock<'a>(db: &'a DbState) -> Result<std::sync::MutexGuard<'a, rusqlite::Connection>, AppError> {
    db.0.lock().map_err(|e| AppError::Database(e.to_string()))
}

pub async fn start_adaptive_search(db: &DbState, input: AdaptiveInput) -> Result<i64, AppError> {
    if input.api_key.trim().is_empty() {
        return Err(AppError::MissingApiKey);
    }
    if input.query.trim().is_empty() || input.city.trim().is_empty() {
        return Err(AppError::InvalidRequest("query and city are required".into()));
    }
    let (lat, lng) = geocode_city(&input.city).await?;
    let region = GeoRegion::from_center(lat, lng, input.radius_meters);
    let job_id = {
        let conn = lock(db)?;
        let id = create_adaptive_job(&conn, &input.query, &input.city, Some(input.radius_meters), lat, lng)?;
        create_cell(&conn, id, None, &region, &input.query)?;
        id
    };
    run_pending_cells(db, job_id, &input.query, &input.api_key, &SplitPolicy::default()).await?;
    Ok(job_id)
}

pub async fn resume_adaptive_search(
    db: &DbState,
    job_id: i64,
    api_key: &str,
) -> Result<(), AppError> {
    if api_key.trim().is_empty() {
        return Err(AppError::MissingApiKey);
    }
    let (query, status) = {
        let conn = lock(db)?;
        let (q, s): (String, String) = conn.query_row(
            "SELECT query, status FROM search_jobs WHERE id=?1",
            rusqlite::params![job_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        (q, s)
    };
    if status != "paused" && status != "interrupted" && status != "failed" {
        return Err(AppError::InvalidRequest(format!("job is {status}, cannot resume")));
    }
    {
        let conn = lock(db)?;
        set_job_status(&conn, job_id, "running", None)?;
        conn.execute(
            "UPDATE search_cells SET status='pending' WHERE job_id=?1 AND status='interrupted'",
            rusqlite::params![job_id],
        )?;
    }
    run_pending_cells(db, job_id, &query, api_key, &SplitPolicy::default()).await
}

pub async fn run_pending_cells(
    db: &DbState,
    job_id: i64,
    query: &str,
    api_key: &str,
    policy: &SplitPolicy,
) -> Result<(), AppError> {
    let provider = GooglePlacesProvider::new();
    loop {
        let status = {
            let guard = lock(db)?;
            guard.let_status(job_id)?
        };
        if status == "paused" || status == "cancelled" {
            return Ok(());
        }
        if status == "failed" {
            return Ok(());
        }
        let cell = {
            let guard = lock(db)?;
            pop_pending_cell(&guard, job_id)?
        };
        let Some(cell) = cell else {
            let conn = lock(db)?;
            aggregate_job_counts(&conn, job_id)?;
            set_job_status(&conn, job_id, "completed", None)?;
            return Ok(());
        };
        {
            let conn = lock(db)?;
            conn.execute("UPDATE search_cells SET status='running' WHERE id=?1", rusqlite::params![cell.id])?;
        }
        let region = GeoRegion { north: cell.north, south: cell.south, east: cell.east, west: cell.west, depth: cell.depth as u8 };
        let (clat, clng) = region.center();
        let radius = region.radius_meters().max(300.0);
        let req = RegionSearchRequest { query: query.to_string(), api_key: api_key.to_string(), center_lat: clat, center_lng: clng, radius_meters: radius };
        let places = match provider.search_region(&req).await {
            Ok(p) => p,
            Err(e) => {
                if matches!(e, AppError::RateLimit) {
                    {
                        let conn = lock(db)?;
                        conn.execute("UPDATE search_cells SET status='pending' WHERE id=?1", rusqlite::params![cell.id])?;
                    }
                    sleep_ms(2000).await;
                    continue;
                }
                if matches!(e, AppError::InvalidApiKey | AppError::MissingApiKey) {
                    {
                        let conn = lock(db)?;
                        mark_cell(&conn, cell.id, "failed", 0, 0)?;
                        set_job_status(&conn, job_id, "failed", Some(&e.to_string()))?;
                    }
                    return Err(e);
                }
                {
                    let conn = lock(db)?;
                    mark_cell(&conn, cell.id, "failed", 0, 0)?;
                }
                sleep_ms(500).await;
                continue;
            }
        };
        let (raw, uniq) = {
            let conn = lock(db)?;
            let mut u = 0i64;
            for p in &places {
                let (_, is_new) = upsert_lead_in_cell(&conn, job_id, Some(cell.id), p)?;
                if is_new {
                    u += 1;
                }
            }
            (places.len() as i64, u)
        };
        let decision = should_split(policy, &region, raw as usize, uniq as usize);
        {
            let conn = lock(db)?;
            match decision {
                SplitDecision::Split => {
                    mark_cell(&conn, cell.id, "split", raw, uniq)?;
                    for child in region.split_into_four() {
                        create_cell(&conn, job_id, Some(cell.id), &child, query)?;
                    }
                }
                SplitDecision::Complete | SplitDecision::StopDepth | SplitDecision::StopSmall | SplitDecision::StopLowGain => {
                    let s = if decision == SplitDecision::Complete { "completed" } else { "saturated" };
                    mark_cell(&conn, cell.id, s, raw, uniq)?;
                }
                SplitDecision::Saturated => {
                    mark_cell(&conn, cell.id, "saturated", raw, uniq)?;
                }
            }
            aggregate_job_counts(&conn, job_id)?;
        }
        sleep_ms(400).await;
    }
}

trait ConnExt {
    fn let_status(&self, job_id: i64) -> rusqlite::Result<String>;
}

impl ConnExt for rusqlite::Connection {
    fn let_status(&self, job_id: i64) -> rusqlite::Result<String> {
        get_job_status(self, job_id)
    }
}

pub fn job_progress_cells(db: &DbState, job_id: i64) -> Result<Vec<crate::database::repositories::SearchCell>, AppError> {
    let guard = lock(db)?;
    Ok(list_cells(&guard, job_id)?)
}
