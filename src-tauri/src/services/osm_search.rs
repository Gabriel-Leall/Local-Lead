use crate::database::repositories::{create_job, finish_job, upsert_scraper_place};
use crate::database::DbState;
use crate::error::AppError;
use crate::providers::osm::OsmProvider;
use crate::services::geocode::geocode_city;

fn lock(db: &DbState) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, AppError> {
    db.0.lock().map_err(|e| AppError::Database(e.to_string()))
}

/// Busca no mapa aberto (OpenStreetMap): geocodifica, consulta o Overpass,
/// persiste com a mesma deduplicação cross-provider. Sem chave, sem binário.
pub async fn run_osm_search(
    db: &DbState,
    query: String,
    city: String,
    radius_meters: f64,
) -> Result<(i64, i64, i64), AppError> {
    if query.trim().is_empty() || city.trim().is_empty() {
        return Err(AppError::InvalidRequest("informe nicho e cidade".into()));
    }
    if !(500.0..=100_000.0).contains(&radius_meters) {
        return Err(AppError::InvalidRequest("raio entre 0,5 e 100 km".into()));
    }
    let (lat, lng) = geocode_city(&city).await?;
    let job_id = {
        let conn = lock(db)?;
        let id = create_job(&conn, &format!("{query} (mapa)"), &city, Some(radius_meters))?;
        conn.execute(
            "UPDATE search_jobs SET strategy='osm', center_lat=?1, center_lng=?2 WHERE id=?3",
            rusqlite::params![lat, lng, id],
        )?;
        id
    };

    let provider = OsmProvider::new();
    let places = match provider.search_at(&query, lat, lng, radius_meters).await {
        Ok(p) => p,
        Err(e) => {
            let conn = lock(db)?;
            finish_job(&conn, job_id, "failed", 0, 0, Some(&e.to_string()))?;
            return Err(e);
        }
    };

    let mut new_count = 0i64;
    {
        let conn = lock(db)?;
        for place in &places {
            let (_, is_new) = upsert_scraper_place(&conn, job_id, None, place)?;
            if is_new {
                new_count += 1;
            }
        }
        finish_job(&conn, job_id, "completed", places.len() as i64, new_count, None)?;
    }
    Ok((job_id, places.len() as i64, new_count))
}
