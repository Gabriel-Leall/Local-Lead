use crate::database::repositories::{list_leads_filtered, LeadFilters};
use crate::database::DbState;
use crate::domain::Lead;
use crate::error::AppError;
use crate::services::enrichment::{enrich_leads, enrich_websites};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct EnrichResponse {
    pub enriched: i64,
    pub failed: i64,
}

#[tauri::command]
pub async fn enrich_leads_cmd(
    db: State<'_, DbState>,
    lead_ids: Vec<i64>,
    api_key: String,
) -> Result<EnrichResponse, AppError> {
    let r = enrich_leads(&db, lead_ids, api_key).await?;
    Ok(EnrichResponse { enriched: r.enriched, failed: r.failed })
}

#[tauri::command]
pub async fn enrich_websites_cmd(
    db: State<'_, DbState>,
    lead_ids: Vec<i64>,
) -> Result<EnrichResponse, AppError> {
    let r = enrich_websites(&db, lead_ids).await?;
    Ok(EnrichResponse { enriched: r.enriched, failed: r.failed })
}

#[tauri::command]
pub async fn enrich_ddg_cmd(
    db: State<'_, DbState>,
    lead_ids: Vec<i64>,
) -> Result<EnrichResponse, AppError> {
    let r = crate::services::enrichment::enrich_via_ddg(&db, lead_ids).await?;
    Ok(EnrichResponse { enriched: r.enriched, failed: r.failed })
}

#[tauri::command]
pub fn get_leads_filtered(
    db: State<'_, DbState>,
    name_filter: Option<String>,
    category_filter: Option<String>,
    status_filter: Option<String>,
    has_website: Option<bool>,
    has_phone: Option<bool>,
    has_email: Option<bool>,
    has_instagram: Option<bool>,
    website_status: Option<String>,
    min_rating: Option<f64>,
    min_reviews: Option<i64>,
    min_score: Option<i64>,
    order_by_score: Option<bool>,
    limit: Option<i64>,
) -> Result<Vec<Lead>, AppError> {
    let conn = db.0.lock().map_err(|e| AppError::Database(e.to_string()))?;
    Ok(list_leads_filtered(
        &conn,
        &LeadFilters {
            name: name_filter.as_deref(),
            category: category_filter.as_deref(),
            status: status_filter.as_deref(),
            has_website,
            has_phone,
            has_email,
            has_instagram,
            website_status: website_status.as_deref(),
            min_rating,
            min_reviews,
            min_score,
            order_by_score: order_by_score.unwrap_or(false),
        },
        limit.unwrap_or(300),
    )?)
}
