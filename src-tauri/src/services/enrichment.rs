use crate::database::repositories::{apply_place_details, apply_website_enrichment, get_config, lead_external_id, lead_website, record_enrichment, record_enrichment_typed, update_lead_score};
use crate::database::DbState;
use crate::enrichment::crawl_website;
use crate::error::AppError;
use crate::providers::GooglePlacesProvider;
use crate::services::lead_scoring::{compute_score, LeadView, ScoreConfig};

fn load_cfg(conn: &rusqlite::Connection) -> ScoreConfig {
    get_config(conn, "score_config")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn refresh_score(conn: &rusqlite::Connection, lead_id: i64) {
    let row: Option<(Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<f64>, Option<i64>)> = conn
        .query_row(
            "SELECT website, website_status, instagram, phone, email, rating, review_count FROM leads WHERE id=?1",
            rusqlite::params![lead_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .ok();
    if let Some((website, ws, ig, phone, email, rating, reviews)) = row {
        let cfg = load_cfg(conn);
        let view = LeadView {
            website: website.as_deref(),
            website_status: ws.as_deref(),
            instagram: ig.as_deref(),
            phone: phone.as_deref(),
            email: email.as_deref(),
            rating,
            review_count: reviews,
        };
        let (score, reasons) = compute_score(&cfg, &view);
        let json = serde_json::to_string(&reasons).unwrap_or_else(|_| "[]".into());
        let _ = update_lead_score(conn, lead_id, score, &json);
    }
}

async fn sleep_ms(ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

fn lock(db: &DbState) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, AppError> {
    db.0.lock().map_err(|e| AppError::Database(e.to_string()))
}

pub struct EnrichResult {
    pub enriched: i64,
    pub failed: i64,
}

pub async fn enrich_leads(
    db: &DbState,
    lead_ids: Vec<i64>,
    api_key: String,
) -> Result<EnrichResult, AppError> {
    if api_key.trim().is_empty() {
        return Err(AppError::MissingApiKey);
    }
    if lead_ids.is_empty() {
        return Err(AppError::InvalidRequest("no leads selected".into()));
    }
    if lead_ids.len() > 100 {
        return Err(AppError::InvalidRequest("max 100 leads per batch".into()));
    }
    let provider = GooglePlacesProvider::new();
    let mut enriched = 0i64;
    let mut failed = 0i64;
    for lead_id in lead_ids {
        let place_id: Option<String> = {
            let guard = lock(db)?;
            lead_external_id(&guard, lead_id)?
        };
        let Some(pid) = place_id else {
            {
                let conn = lock(db)?;
                record_enrichment(&conn, lead_id, "failed", Some("no external place id"))?;
            }
            failed += 1;
            continue;
        };
        match provider.fetch_details(&pid, &api_key).await {
            Ok(d) => {
                {
                    let conn = lock(db)?;
                    apply_place_details(
                        &conn,
                        lead_id,
                        d.phone.as_deref(),
                        d.website.as_deref(),
                        d.rating,
                        d.review_count,
                        d.address.as_deref(),
                        d.category.as_deref(),
                    )?;
                    refresh_score(&conn, lead_id);
                    record_enrichment(&conn, lead_id, "completed", None)?;
                }
                enriched += 1;
            }
            Err(e) => {
                if matches!(e, AppError::RateLimit) {
                    sleep_ms(2000).await;
                }
                if matches!(e, AppError::InvalidApiKey) {
                    return Err(e);
                }
                {
                    let conn = lock(db)?;
                    record_enrichment(&conn, lead_id, "failed", Some(&e.to_string()))?;
                }
                failed += 1;
            }
        }
        sleep_ms(300).await;
    }
    Ok(EnrichResult { enriched, failed })
}

pub async fn enrich_websites(db: &DbState, lead_ids: Vec<i64>) -> Result<EnrichResult, AppError> {
    if lead_ids.is_empty() {
        return Err(AppError::InvalidRequest("no leads selected".into()));
    }
    if lead_ids.len() > 50 {
        return Err(AppError::InvalidRequest("max 50 leads per batch".into()));
    }
    let mut enriched = 0i64;
    let mut failed = 0i64;
    for lead_id in lead_ids {
        let website: Option<String> = {
            let guard = lock(db)?;
            lead_website(&guard, lead_id)?
        };
        let Some(site) = website.filter(|s| !s.trim().is_empty()) else {
            {
                let conn = lock(db)?;
                apply_website_enrichment(&conn, lead_id, None, None, None, None, None, Some("none"))?;
                record_enrichment_typed(&conn, lead_id, "website", "completed", None)?;
            }
            enriched += 1;
            continue;
        };
        match crawl_website(&site).await {
            Ok(r) => {
                {
                    let conn = lock(db)?;
                    apply_website_enrichment(
                        &conn,
                        lead_id,
                        r.email.as_deref(),
                        r.instagram.as_deref(),
                        r.facebook.as_deref(),
                        r.whatsapp.as_deref(),
                        r.phone.as_deref(),
                        r.status.as_deref(),
                    )?;
                    refresh_score(&conn, lead_id);
                    record_enrichment_typed(&conn, lead_id, "website", "completed", None)?;
                }
                enriched += 1;
            }
            Err(e) => {
                {
                    let conn = lock(db)?;
                    record_enrichment_typed(&conn, lead_id, "website", "failed", Some(&e.to_string()))?;
                }
                failed += 1;
            }
        }
        sleep_ms(400).await;
    }
    Ok(EnrichResult { enriched, failed })
}
