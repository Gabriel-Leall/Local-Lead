use super::fingerprint::{coords_close, normalize_domain, normalize_name, normalize_phone};
use rusqlite::{params, Connection};

pub enum Match {
    None,
    Lead(i64),
}

/// Cross-provider fallback order: website → phone → name+coords → name+address.
/// Never merges on low confidence: requires exact normalized match at each step.
pub fn find_cross_provider_match(
    conn: &Connection,
    name: &str,
    website: Option<&str>,
    phone: Option<&str>,
    lat: Option<f64>,
    lng: Option<f64>,
    address: Option<&str>,
) -> rusqlite::Result<Match> {
    if let Some(dom) = normalize_domain(website) {
        let mut stmt = conn.prepare("SELECT id, website FROM leads WHERE website IS NOT NULL AND website != '' LIMIT 500")?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        for (id, w) in rows {
            if normalize_domain(Some(&w)).as_deref() == Some(dom.as_str()) {
                return Ok(Match::Lead(id));
            }
        }
    }
    if let Some(ph) = normalize_phone(phone) {
        let mut stmt = conn.prepare("SELECT id, phone FROM leads WHERE phone IS NOT NULL AND phone != '' LIMIT 500")?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        for (id, p) in rows {
            if normalize_phone(Some(&p)).as_deref() == Some(ph.as_str()) {
                return Ok(Match::Lead(id));
            }
        }
    }
    let norm = normalize_name(name);
    if !norm.is_empty() {
        let mut stmt = conn.prepare("SELECT id, canonical_name, latitude, longitude, address FROM leads LIMIT 1000")?;
        let rows: Vec<(i64, String, Option<f64>, Option<f64>, Option<String>)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
            .collect::<Result<_, _>>()?;
        for (id, existing_name, elat, elng, eaddr) in rows {
            if normalize_name(&existing_name) != norm {
                continue;
            }
            if coords_close(lat, lng, elat, elng) {
                return Ok(Match::Lead(id));
            }
            if let (Some(a), Some(b)) = (address, eaddr.as_deref()) {
                if normalize_name(a) == normalize_name(b) && !a.trim().is_empty() {
                    return Ok(Match::Lead(id));
                }
            }
        }
    }
    let _ = params![];
    Ok(Match::None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_db;

    #[test]
    fn no_match_on_empty_db() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let m = find_cross_provider_match(&conn, "Miami Dental", None, None, None, None, None).unwrap();
        assert!(matches!(m, Match::None));
    }
}
