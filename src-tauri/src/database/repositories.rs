use crate::domain::{DiscoveredPlace, GeoRegion, Lead};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::Serialize;

fn now() -> String {
    Utc::now().to_rfc3339()
}

pub fn create_job(
    conn: &Connection,
    query: &str,
    city: &str,
    radius_meters: Option<f64>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO search_jobs (query, city, radius_meters, status, created_at) VALUES (?1,?2,?3,'running',?4)",
        params![query, city, radius_meters, now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn finish_job(
    conn: &Connection,
    job_id: i64,
    status: &str,
    result_count: i64,
    new_count: i64,
    error: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE search_jobs SET status=?1, result_count=?2, new_count=?3, error=?4, completed_at=?5 WHERE id=?6",
        params![status, result_count, new_count, error, now(), job_id],
    )?;
    Ok(())
}

pub fn create_adaptive_job(
    conn: &Connection,
    query: &str,
    city: &str,
    radius_meters: Option<f64>,
    center_lat: f64,
    center_lng: f64,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO search_jobs (query, city, radius_meters, center_lat, center_lng, strategy, status, created_at) VALUES (?1,?2,?3,?4,?5,'adaptive','running',?6)",
        params![query, city, radius_meters, center_lat, center_lng, now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_job_status(conn: &Connection, job_id: i64) -> rusqlite::Result<String> {
    conn.query_row("SELECT status FROM search_jobs WHERE id=?1", params![job_id], |r| r.get(0))
}

pub fn set_job_status(
    conn: &Connection,
    job_id: i64,
    status: &str,
    error: Option<&str>,
) -> rusqlite::Result<()> {
    if status == "completed" || status == "failed" || status == "cancelled" {
        conn.execute(
            "UPDATE search_jobs SET status=?1, error=?2, completed_at=?3 WHERE id=?4",
            params![status, error, now(), job_id],
        )?;
    } else {
        conn.execute(
            "UPDATE search_jobs SET status=?1 WHERE id=?2",
            params![status, job_id],
        )?;
    }
    Ok(())
}

pub fn aggregate_job_counts(conn: &Connection, job_id: i64) -> rusqlite::Result<(i64, i64)> {
    let result_count: i64 = conn.query_row(
        "SELECT COALESCE(SUM(raw_result_count),0) FROM search_cells WHERE job_id=?1 AND status IN ('completed','saturated','split')",
        params![job_id],
        |r| r.get(0),
    )?;
    let new_count: i64 = conn.query_row(
        "SELECT COALESCE(SUM(new_unique_count),0) FROM search_cells WHERE job_id=?1",
        params![job_id],
        |r| r.get(0),
    )?;
    conn.execute(
        "UPDATE search_jobs SET result_count=?1, new_count=?2 WHERE id=?3",
        params![result_count, new_count, job_id],
    )?;
    Ok((result_count, new_count))
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchCell {
    pub id: i64,
    pub job_id: i64,
    pub parent_id: Option<i64>,
    pub north: f64,
    pub south: f64,
    pub east: f64,
    pub west: f64,
    pub depth: i64,
    pub status: String,
    pub raw_result_count: i64,
    pub new_unique_count: i64,
    pub query: String,
    pub provider: String,
}

pub fn create_cell(
    conn: &Connection,
    job_id: i64,
    parent_id: Option<i64>,
    region: &GeoRegion,
    query: &str,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO search_cells (job_id, parent_id, north, south, east, west, depth, status, query, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,'pending',?8,?9)",
        params![job_id, parent_id, region.north, region.south, region.east, region.west, region.depth as i64, query, now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_cells(conn: &Connection, job_id: i64) -> rusqlite::Result<Vec<SearchCell>> {
    let mut stmt = conn.prepare(
        "SELECT id, job_id, parent_id, north, south, east, west, depth, status, raw_result_count, new_unique_count, query, provider FROM search_cells WHERE job_id=?1 ORDER BY id",
    )?;
    let rows = stmt.query_map(params![job_id], |r| {
        Ok(SearchCell {
            id: r.get(0)?,
            job_id: r.get(1)?,
            parent_id: r.get(2)?,
            north: r.get(3)?,
            south: r.get(4)?,
            east: r.get(5)?,
            west: r.get(6)?,
            depth: r.get(7)?,
            status: r.get(8)?,
            raw_result_count: r.get(9)?,
            new_unique_count: r.get(10)?,
            query: r.get(11)?,
            provider: r.get(12)?,
        })
    })?;
    rows.collect()
}

pub fn pop_pending_cell(conn: &Connection, job_id: i64) -> rusqlite::Result<Option<SearchCell>> {
    let opt: Option<SearchCell> = conn
        .query_row(
            "SELECT id, job_id, parent_id, north, south, east, west, depth, status, raw_result_count, new_unique_count, query, provider FROM search_cells WHERE job_id=?1 AND status='pending' ORDER BY depth, id LIMIT 1",
            params![job_id],
            |r| {
                Ok(SearchCell {
                    id: r.get(0)?,
                    job_id: r.get(1)?,
                    parent_id: r.get(2)?,
                    north: r.get(3)?,
                    south: r.get(4)?,
                    east: r.get(5)?,
                    west: r.get(6)?,
                    depth: r.get(7)?,
                    status: r.get(8)?,
                    raw_result_count: r.get(9)?,
                    new_unique_count: r.get(10)?,
                    query: r.get(11)?,
                    provider: r.get(12)?,
                })
            },
        )
        .ok();
    Ok(opt)
}

pub fn mark_cell(
    conn: &Connection,
    cell_id: i64,
    status: &str,
    raw: i64,
    uniq: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE search_cells SET status=?1, raw_result_count=?2, new_unique_count=?3, completed_at=?4 WHERE id=?5",
        params![status, raw, uniq, now(), cell_id],
    )?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct JobSummary {
    pub id: i64,
    pub query: String,
    pub city: String,
    pub radius_meters: Option<f64>,
    pub center_lat: Option<f64>,
    pub center_lng: Option<f64>,
    pub strategy: String,
    pub status: String,
    pub result_count: i64,
    pub new_count: i64,
    pub total_cells: i64,
    pub completed_cells: i64,
    pub coverage: f64,
    pub created_at: String,
}

pub fn list_jobs(conn: &Connection) -> rusqlite::Result<Vec<JobSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, query, city, radius_meters, center_lat, center_lng, COALESCE(strategy,'single'), status, result_count, new_count, created_at FROM search_jobs ORDER BY id DESC LIMIT 50",
    )?;
    let jobs: Vec<(i64, String, String, Option<f64>, Option<f64>, Option<f64>, String, String, i64, i64, String)> =
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?)))?.collect::<Result<_,_>>()?;
    let mut out = Vec::new();
    for (id, query, city, radius, clat, clng, strategy, status, rc, nc, created) in jobs {
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM search_cells WHERE job_id=?1", params![id], |r| r.get(0)).unwrap_or(0);
        let done: i64 = conn.query_row("SELECT COUNT(*) FROM search_cells WHERE job_id=?1 AND status IN ('completed','saturated','split','failed')", params![id], |r| r.get(0)).unwrap_or(0);
        let coverage = if total == 0 { if status == "completed" { 1.0 } else { 0.0 } } else { done as f64 / total as f64 };
        out.push(JobSummary { id, query, city, radius_meters: radius, center_lat: clat, center_lng: clng, strategy, status, result_count: rc, new_count: nc, total_cells: total, completed_cells: done, coverage, created_at: created });
    }
    Ok(out)
}

pub fn upsert_lead(
    conn: &Connection,
    job_id: i64,
    place: &DiscoveredPlace,
) -> rusqlite::Result<(i64, bool)> {
    upsert_lead_in_cell(conn, job_id, None, place)
}

pub fn upsert_lead_in_cell(
    conn: &Connection,
    job_id: i64,
    cell_id: Option<i64>,
    place: &DiscoveredPlace,
) -> rusqlite::Result<(i64, bool)> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT lead_id FROM external_places WHERE provider=?1 AND external_place_id=?2",
            params![place.provider, place.external_id],
            |r| r.get(0),
        )
        .ok();

    if let Some(lead_id) = existing {
        conn.execute(
            "UPDATE leads SET category=COALESCE(?1,category), address=COALESCE(?2,address), latitude=COALESCE(?3,latitude), longitude=COALESCE(?4,longitude), phone=COALESCE(?5,phone), website=COALESCE(?6,website), rating=COALESCE(?7,rating), review_count=COALESCE(?8,review_count), instagram=COALESCE(?9,instagram), facebook=COALESCE(?10,facebook), updated_at=?11 WHERE id=?12",
            params![
                place.category,
                place.address,
                place.latitude,
                place.longitude,
                place.phone,
                place.website,
                place.rating,
                place.review_count,
                place.instagram,
                place.facebook,
                now(),
                lead_id
            ],
        )?;
        return Ok((lead_id, false));
    }

    conn.execute(
        "INSERT INTO leads (canonical_name, category, address, latitude, longitude, phone, website, rating, review_count, instagram, facebook, lead_status, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'new',?12,?12)",
        params![
            place.name,
            place.category,
            place.address,
            place.latitude,
            place.longitude,
            place.phone,
            place.website,
            place.rating,
            place.review_count,
            place.instagram,
            place.facebook,
            now()
        ],
    )?;
    let lead_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO external_places (lead_id, provider, external_place_id, raw_name, raw_address, discovered_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![lead_id, place.provider, place.external_id, place.name, place.address, now()],
    )?;
    conn.execute(
        "INSERT INTO discoveries (lead_id, job_id, cell_id, provider, query, discovered_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![lead_id, job_id, cell_id, place.provider, place.name, now()],
    )?;
    Ok((lead_id, true))
}

pub struct LeadFilters<'a> {
    pub name: Option<&'a str>,
    pub category: Option<&'a str>,
    pub status: Option<&'a str>,
    pub has_website: Option<bool>,
    pub has_phone: Option<bool>,
    pub has_email: Option<bool>,
    pub has_instagram: Option<bool>,
    pub website_status: Option<&'a str>,
    pub min_rating: Option<f64>,
    pub min_reviews: Option<i64>,
    pub min_score: Option<i64>,
    pub order_by_score: bool,
}

pub fn list_leads(
    conn: &Connection,
    name_filter: Option<&str>,
    category_filter: Option<&str>,
    status_filter: Option<&str>,
    limit: i64,
) -> rusqlite::Result<Vec<Lead>> {
    list_leads_filtered(
        conn,
        &LeadFilters { name: name_filter, category: category_filter, status: status_filter, has_website: None, has_phone: None, has_email: None, has_instagram: None, website_status: None, min_rating: None, min_reviews: None, min_score: None, order_by_score: false },
        limit,
    )
}

pub fn list_leads_filtered(
    conn: &Connection,
    f: &LeadFilters,
    limit: i64,
) -> rusqlite::Result<Vec<Lead>> {
    let mut sql = String::from(
        "SELECT id, canonical_name, category, address, latitude, longitude, phone, website, rating, review_count, email, instagram, facebook, whatsapp, website_status, score, score_reasons, lead_status, follow_up_at, created_at, updated_at FROM leads WHERE 1=1",
    );
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(n) = f.name {
        sql.push_str(" AND canonical_name LIKE ?");
        args.push(Box::new(format!("%{n}%")));
    }
    if let Some(c) = f.category {
        sql.push_str(" AND category = ?");
        args.push(Box::new(c.to_owned()));
    }
    if let Some(s) = f.status {
        sql.push_str(" AND lead_status = ?");
        args.push(Box::new(s.to_owned()));
    }
    if let Some(hw) = f.has_website {
        if hw {
            sql.push_str(" AND website IS NOT NULL AND website != ''");
        } else {
            sql.push_str(" AND (website IS NULL OR website = '')");
        }
    }
    if let Some(hp) = f.has_phone {
        if hp {
            sql.push_str(" AND phone IS NOT NULL AND phone != ''");
        } else {
            sql.push_str(" AND (phone IS NULL OR phone = '')");
        }
    }
    if let Some(he) = f.has_email {
        if he {
            sql.push_str(" AND email IS NOT NULL AND email != ''");
        } else {
            sql.push_str(" AND (email IS NULL OR email = '')");
        }
    }
    if let Some(hi) = f.has_instagram {
        if hi {
            sql.push_str(" AND instagram IS NOT NULL AND instagram != ''");
        } else {
            sql.push_str(" AND (instagram IS NULL OR instagram = '')");
        }
    }
    if let Some(ws) = f.website_status {
        sql.push_str(" AND website_status = ?");
        args.push(Box::new(ws.to_owned()));
    }
    if let Some(mr) = f.min_rating {
        sql.push_str(" AND rating >= ?");
        args.push(Box::new(mr));
    }
    if let Some(mrev) = f.min_reviews {
        sql.push_str(" AND review_count >= ?");
        args.push(Box::new(mrev));
    }
    if let Some(ms) = f.min_score {
        sql.push_str(" AND score >= ?");
        args.push(Box::new(ms));
    }
    if f.order_by_score {
        sql.push_str(" ORDER BY score DESC, id DESC LIMIT ?");
    } else {
        sql.push_str(" ORDER BY id DESC LIMIT ?");
    }
    args.push(Box::new(limit));

    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
    let rows = stmt.query_map(params.as_slice(), |r| {
        Ok(Lead {
            id: r.get(0)?,
            canonical_name: r.get(1)?,
            category: r.get(2)?,
            address: r.get(3)?,
            latitude: r.get(4)?,
            longitude: r.get(5)?,
            phone: r.get(6)?,
            website: r.get(7)?,
            rating: r.get(8)?,
            review_count: r.get(9)?,
            email: r.get(10)?,
            instagram: r.get(11)?,
            facebook: r.get(12)?,
            whatsapp: r.get(13)?,
            website_status: r.get(14)?,
            score: r.get(15)?,
            score_reasons: r.get(16)?,
            lead_status: r.get(17)?,
            follow_up_at: r.get(18)?,
            created_at: r.get(19)?,
            updated_at: r.get(20)?,
        })
    })?;
    rows.collect()
}

pub fn change_lead_status(
    conn: &Connection,
    lead_id: i64,
    to_status: &str,
    note: Option<&str>,
) -> rusqlite::Result<()> {
    let from: Option<String> = conn
        .query_row("SELECT lead_status FROM leads WHERE id=?1", params![lead_id], |r| r.get(0))
        .ok();
    conn.execute(
        "UPDATE leads SET lead_status=?1, updated_at=?2 WHERE id=?3",
        params![to_status, now(), lead_id],
    )?;
    conn.execute(
        "INSERT INTO lead_status_history (lead_id, from_status, to_status, note, created_at) VALUES (?1,?2,?3,?4,?5)",
        params![lead_id, from, to_status, note, now()],
    )?;
    Ok(())
}

pub fn add_lead_note(
    conn: &Connection,
    lead_id: i64,
    note: &str,
    follow_up_at: Option<&str>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO lead_notes (lead_id, note, follow_up_at, created_at) VALUES (?1,?2,?3,?4)",
        params![lead_id, note, follow_up_at, now()],
    )?;
    if let Some(fu) = follow_up_at {
        conn.execute("UPDATE leads SET follow_up_at=?1, updated_at=?2 WHERE id=?3", params![fu, now(), lead_id])?;
    }
    Ok(conn.last_insert_rowid())
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusEvent {
    pub id: i64,
    pub from_status: Option<String>,
    pub to_status: String,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LeadNote {
    pub id: i64,
    pub note: String,
    pub follow_up_at: Option<String>,
    pub created_at: String,
}

pub fn list_history(conn: &Connection, lead_id: i64) -> rusqlite::Result<Vec<StatusEvent>> {
    let mut stmt = conn.prepare(
        "SELECT id, from_status, to_status, note, created_at FROM lead_status_history WHERE lead_id=?1 ORDER BY id DESC LIMIT 100",
    )?;
    let rows = stmt.query_map(params![lead_id], |r| {
        Ok(StatusEvent { id: r.get(0)?, from_status: r.get(1)?, to_status: r.get(2)?, note: r.get(3)?, created_at: r.get(4)? })
    })?;
    rows.collect()
}

pub fn list_notes(conn: &Connection, lead_id: i64) -> rusqlite::Result<Vec<LeadNote>> {
    let mut stmt = conn.prepare(
        "SELECT id, note, follow_up_at, created_at FROM lead_notes WHERE lead_id=?1 ORDER BY id DESC LIMIT 100",
    )?;
    let rows = stmt.query_map(params![lead_id], |r| {
        Ok(LeadNote { id: r.get(0)?, note: r.get(1)?, follow_up_at: r.get(2)?, created_at: r.get(3)? })
    })?;
    rows.collect()
}

pub fn lead_external_id(conn: &Connection, lead_id: i64) -> rusqlite::Result<Option<String>> {
    let r: Option<String> = conn
        .query_row(
            "SELECT external_place_id FROM external_places WHERE lead_id=?1 AND provider='google_places' ORDER BY id LIMIT 1",
            params![lead_id],
            |row| row.get(0),
        )
        .ok();
    Ok(r)
}

pub fn apply_place_details(
    conn: &Connection,
    lead_id: i64,
    phone: Option<&str>,
    website: Option<&str>,
    rating: Option<f64>,
    review_count: Option<i64>,
    address: Option<&str>,
    category: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE leads SET phone=COALESCE(?1,phone), website=COALESCE(?2,website), rating=COALESCE(?3,rating), review_count=COALESCE(?4,review_count), address=COALESCE(?5,address), category=COALESCE(?6,category), updated_at=?7 WHERE id=?8",
        params![phone, website, rating, review_count, address, category, now(), lead_id],
    )?;
    Ok(())
}

pub fn record_enrichment(
    conn: &Connection,
    lead_id: i64,
    status: &str,
    error: Option<&str>,
) -> rusqlite::Result<()> {
    record_enrichment_typed(conn, lead_id, "place_details", status, error)
}

pub fn record_enrichment_typed(
    conn: &Connection,
    lead_id: i64,
    enrichment_type: &str,
    status: &str,
    error: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO enrichment_jobs (lead_id, enrichment_type, status, attempts, error, created_at, updated_at) VALUES (?1,?2,?3,1,?4,?5,?5)",
        params![lead_id, enrichment_type, status, error, now()],
    )?;
    Ok(())
}

pub fn lead_website(conn: &Connection, lead_id: i64) -> rusqlite::Result<Option<String>> {
    let r: Option<String> = conn
        .query_row("SELECT website FROM leads WHERE id=?1", params![lead_id], |row| row.get(0))
        .ok()
        .flatten();
    Ok(r)
}

pub fn apply_website_enrichment(
    conn: &Connection,
    lead_id: i64,
    email: Option<&str>,
    instagram: Option<&str>,
    facebook: Option<&str>,
    whatsapp: Option<&str>,
    phone: Option<&str>,
    website_status: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE leads SET email=COALESCE(?1,email), instagram=COALESCE(?2,instagram), facebook=COALESCE(?3,facebook), whatsapp=COALESCE(?4,whatsapp), phone=COALESCE(?5,phone), website_status=COALESCE(?6,website_status), updated_at=?7 WHERE id=?8",
        params![email, instagram, facebook, whatsapp, phone, website_status, now(), lead_id],
    )?;
    Ok(())
}

pub fn update_lead_score(
    conn: &Connection,
    lead_id: i64,
    score: i64,
    reasons_json: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE leads SET score=?1, score_reasons=?2, updated_at=?3 WHERE id=?4",
        params![score, reasons_json, now(), lead_id],
    )?;
    Ok(())
}

pub fn get_config(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM app_config WHERE key=?1", params![key], |r| r.get(0))
        .ok())
}

pub fn set_config(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO app_config (key, value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedFilter {
    pub id: i64,
    pub name: String,
    pub filters_json: String,
    pub created_at: String,
}

pub fn list_saved_filters(conn: &Connection) -> rusqlite::Result<Vec<SavedFilter>> {
    let mut stmt = conn.prepare("SELECT id, name, filters_json, created_at FROM saved_filters ORDER BY name")?;
    let rows = stmt.query_map([], |r| {
        Ok(SavedFilter { id: r.get(0)?, name: r.get(1)?, filters_json: r.get(2)?, created_at: r.get(3)? })
    })?;
    rows.collect()
}

pub fn save_filter(conn: &Connection, name: &str, filters_json: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO saved_filters (name, filters_json, created_at) VALUES (?1,?2,?3) ON CONFLICT(name) DO UPDATE SET filters_json=excluded.filters_json",
        params![name, filters_json, now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_saved_filter(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM saved_filters WHERE id=?1", params![id])?;
    Ok(())
}

pub fn upsert_scraper_place(
    conn: &Connection,
    job_id: i64,
    cell_id: Option<i64>,
    place: &DiscoveredPlace,
) -> rusqlite::Result<(i64, bool)> {
    if let Ok(existing) = conn.query_row(
        "SELECT lead_id FROM external_places WHERE provider=?1 AND external_place_id=?2",
        params![place.provider, place.external_id],
        |r| r.get::<_, i64>(0),
    ) {
        return Ok((existing, false));
    }
    match crate::dedupe::find_cross_provider_match(
        conn,
        &place.name,
        place.website.as_deref(),
        place.phone.as_deref(),
        place.latitude,
        place.longitude,
        place.address.as_deref(),
    )? {
        crate::dedupe::Match::Lead(lead_id) => {
            conn.execute(
                "UPDATE leads SET category=COALESCE(?1,category), address=COALESCE(?2,address), latitude=COALESCE(?3,latitude), longitude=COALESCE(?4,longitude), phone=COALESCE(?5,phone), website=COALESCE(?6,website), rating=COALESCE(?7,rating), review_count=COALESCE(?8,review_count), instagram=COALESCE(?9,instagram), facebook=COALESCE(?10,facebook), updated_at=?11 WHERE id=?12",
                params![place.category, place.address, place.latitude, place.longitude, place.phone, place.website, place.rating, place.review_count, place.instagram, place.facebook, now(), lead_id],
            )?;
            conn.execute(
                "INSERT OR IGNORE INTO external_places (lead_id, provider, external_place_id, raw_name, raw_address, discovered_at) VALUES (?1,?2,?3,?4,?5,?6)",
                params![lead_id, place.provider, place.external_id, place.name, place.address, now()],
            )?;
            conn.execute(
                "INSERT INTO discoveries (lead_id, job_id, cell_id, provider, query, discovered_at) VALUES (?1,?2,?3,?4,?5,?6)",
                params![lead_id, job_id, cell_id, place.provider, place.name, now()],
            )?;
            Ok((lead_id, false))
        }
        crate::dedupe::Match::None => upsert_lead_in_cell(conn, job_id, cell_id, place),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderCount {
    pub provider: String,
    pub leads: i64,
    pub discoveries: i64,
}

pub fn provider_counts(conn: &Connection, job_id: i64) -> rusqlite::Result<Vec<ProviderCount>> {
    let mut stmt = conn.prepare(
        "SELECT provider, COUNT(DISTINCT lead_id), COUNT(*) FROM discoveries WHERE job_id=?1 GROUP BY provider",
    )?;
    let rows = stmt.query_map(params![job_id], |r| {
        Ok(ProviderCount { provider: r.get(0)?, leads: r.get(1)?, discoveries: r.get(2)? })
    })?;
    rows.collect()
}

pub fn get_lead(conn: &Connection, lead_id: i64) -> rusqlite::Result<Option<Lead>> {
    let mut stmt = conn.prepare(
        "SELECT id, canonical_name, category, address, latitude, longitude, phone, website, rating, review_count, email, instagram, facebook, whatsapp, website_status, score, score_reasons, lead_status, follow_up_at, created_at, updated_at FROM leads WHERE id=?1",
    )?;
    let r = stmt
        .query_map(params![lead_id], |r| {
            Ok(Lead {
                id: r.get(0)?,
                canonical_name: r.get(1)?,
                category: r.get(2)?,
                address: r.get(3)?,
                latitude: r.get(4)?,
                longitude: r.get(5)?,
                phone: r.get(6)?,
                website: r.get(7)?,
                rating: r.get(8)?,
                review_count: r.get(9)?,
                email: r.get(10)?,
                instagram: r.get(11)?,
                facebook: r.get(12)?,
                whatsapp: r.get(13)?,
                website_status: r.get(14)?,
                score: r.get(15)?,
                score_reasons: r.get(16)?,
                lead_status: r.get(17)?,
                follow_up_at: r.get(18)?,
                created_at: r.get(19)?,
                updated_at: r.get(20)?,
            })
        })?
        .next()
        .transpose()?;
    Ok(r)
}

#[derive(Debug, Clone, Serialize)]
pub struct OutreachMessage {
    pub id: i64,
    pub lead_id: i64,
    pub channel: String,
    pub subject: Option<String>,
    pub message: String,
    pub template_id: Option<String>,
    pub provider: String,
    pub status: String,
    pub created_at: String,
}

pub fn create_message(
    conn: &Connection,
    lead_id: i64,
    channel: &str,
    subject: Option<&str>,
    message: &str,
    template_id: Option<&str>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO outreach_messages (lead_id, channel, subject, message, template_id, provider, status, created_at) VALUES (?1,?2,?3,?4,?5,'template','generated',?6)",
        params![lead_id, channel, subject, message, template_id, now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_messages(conn: &Connection, lead_id: i64) -> rusqlite::Result<Vec<OutreachMessage>> {
    let mut stmt = conn.prepare(
        "SELECT id, lead_id, channel, subject, message, template_id, provider, status, created_at FROM outreach_messages WHERE lead_id=?1 ORDER BY id DESC",
    )?;
    let rows = stmt.query_map(params![lead_id], |r| {
        Ok(OutreachMessage {
            id: r.get(0)?,
            lead_id: r.get(1)?,
            channel: r.get(2)?,
            subject: r.get(3)?,
            message: r.get(4)?,
            template_id: r.get(5)?,
            provider: r.get(6)?,
            status: r.get(7)?,
            created_at: r.get(8)?,
        })
    })?;
    rows.collect()
}

pub fn set_message_status(conn: &Connection, id: i64, status: &str) -> rusqlite::Result<()> {
    if !["generated", "approved", "sent", "replied", "skipped"].contains(&status) {
        return Err(rusqlite::Error::InvalidParameterName("invalid status".into()));
    }
    if status == "approved" {
        conn.execute("UPDATE outreach_messages SET status='approved', approved_at=?1 WHERE id=?2", params![now(), id])?;
    } else if status == "sent" {
        conn.execute("UPDATE outreach_messages SET status='sent', sent_at=?1 WHERE id=?2", params![now(), id])?;
    } else {
        conn.execute("UPDATE outreach_messages SET status=?1 WHERE id=?2", params![status, id])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_db;

    fn place(id: &str) -> DiscoveredPlace {
        DiscoveredPlace {
            external_id: id.into(),
            name: "Miami Dental".into(),
            category: Some("dentist".into()),
            latitude: Some(25.7),
            longitude: Some(-80.3),
            address: Some("Miami".into()),
            provider: "google_places".into(),
            phone: None,
            website: None,
            rating: Some(4.8),
            review_count: Some(10),
            instagram: None,
            facebook: None,
        }
    }

    #[test]
    fn upsert_does_not_duplicate() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let job = create_job(&conn, "dentist", "Miami", Some(30000.0)).unwrap();
        let (_, is_new1) = upsert_lead(&conn, job, &place("places/1")).unwrap();
        let (_, is_new2) = upsert_lead(&conn, job, &place("places/1")).unwrap();
        assert!(is_new1);
        assert!(!is_new2);
        let leads = list_leads(&conn, None, None, None, 10).unwrap();
        assert_eq!(leads.len(), 1);
    }
}
