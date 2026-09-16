use crate::database::repositories::{create_job, finish_job, upsert_lead};
use crate::database::DbState;
use crate::error::AppError;
use crate::providers::{DiscoveryProvider, GooglePlacesProvider, SearchRequest};

pub struct RunSearchInput {
    pub query: String,
    pub city: String,
    pub radius_meters: Option<f64>,
    pub api_key: String,
}

pub async fn run_search(
    db: &DbState,
    input: RunSearchInput,
) -> Result<(i64, i64, i64), AppError> {
    if input.api_key.trim().is_empty() {
        return Err(AppError::MissingApiKey);
    }
    if input.query.trim().is_empty() || input.city.trim().is_empty() {
        return Err(AppError::InvalidRequest("query and city are required".into()));
    }

    let job_id = {
        let conn = db.0.lock().map_err(|e| AppError::Database(e.to_string()))?;
        create_job(&conn, &input.query, &input.city, input.radius_meters)?
    };

    let provider = GooglePlacesProvider::new();
    let req = SearchRequest {
        query: input.query.clone(),
        city: input.city.clone(),
        api_key: input.api_key,
    };

    let places = match provider.search(&req).await {
        Ok(p) => p,
        Err(e) => {
            let conn = db.0.lock().map_err(|e| AppError::Database(e.to_string()))?;
            finish_job(&conn, job_id, "failed", 0, 0, Some(&e.to_string()))?;
            return Err(e);
        }
    };

    let mut new_count = 0i64;
    {
        let conn = db.0.lock().map_err(|e| AppError::Database(e.to_string()))?;
        for place in &places {
            let (_, is_new) = upsert_lead(&conn, job_id, place)?;
            if is_new {
                new_count += 1;
            }
        }
        finish_job(
            &conn,
            job_id,
            "completed",
            places.len() as i64,
            new_count,
            None,
        )?;
        crate::services::lead_scoring::rescore_with_saved_config(&conn);
    }

    Ok((job_id, places.len() as i64, new_count))
}
