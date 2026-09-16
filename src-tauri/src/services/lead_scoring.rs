use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreConfig {
    pub no_website: i64,
    pub website_offline: i64,
    pub has_instagram: i64,
    pub has_phone: i64,
    pub has_email: i64,
    pub rating_gte: i64,
    pub reviews_gte: i64,
    pub active_bonus: i64,
    pub rating_threshold: f64,
    pub reviews_threshold: i64,
}

impl Default for ScoreConfig {
    fn default() -> Self {
        Self {
            no_website: 30,
            website_offline: 15,
            has_instagram: 10,
            has_phone: 10,
            has_email: 10,
            rating_gte: 10,
            reviews_gte: 10,
            active_bonus: 5,
            rating_threshold: 4.0,
            reviews_threshold: 30,
        }
    }
}

pub struct LeadView<'a> {
    pub website: Option<&'a str>,
    pub website_status: Option<&'a str>,
    pub instagram: Option<&'a str>,
    pub phone: Option<&'a str>,
    pub email: Option<&'a str>,
    pub rating: Option<f64>,
    pub review_count: Option<i64>,
}

/// Recalcula todos os scores com a config salva (ou padrão).
/// Chamado após cada importação para nenhum lead ficar com score 0.
pub fn rescore_with_saved_config(conn: &rusqlite::Connection) {
    let cfg: ScoreConfig = crate::database::repositories::get_config(conn, "score_config")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let _ = rescore_all(conn, &cfg);
}

pub fn rescore_all(
    conn: &rusqlite::Connection,
    cfg: &ScoreConfig,
) -> rusqlite::Result<i64> {
    let mut stmt = conn.prepare(
        "SELECT id, website, website_status, instagram, phone, email, rating, review_count FROM leads",
    )?;
    let rows: Vec<(i64, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<f64>, Option<i64>)> =
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)))?
            .collect::<Result<_, _>>()?;
    let mut n = 0i64;
    for (id, website, ws, ig, phone, email, rating, reviews) in rows {
        let view = LeadView {
            website: website.as_deref(),
            website_status: ws.as_deref(),
            instagram: ig.as_deref(),
            phone: phone.as_deref(),
            email: email.as_deref(),
            rating,
            review_count: reviews,
        };
        let (score, reasons) = compute_score(cfg, &view);
        let json = serde_json::to_string(&reasons).unwrap_or_else(|_| "[]".into());
        crate::database::repositories::update_lead_score(conn, id, score, &json)?;
        n += 1;
    }
    Ok(n)
}

pub fn compute_score(cfg: &ScoreConfig, lead: &LeadView) -> (i64, Vec<String>) {
    let mut score = 0i64;
    let mut reasons = Vec::new();
    let has_site = lead.website.map(|s| !s.trim().is_empty()).unwrap_or(false);

    if !has_site {
        score += cfg.no_website;
        reasons.push("No website".into());
    } else if matches!(lead.website_status, Some("unreachable") | Some("parked")) {
        score += cfg.website_offline;
        reasons.push("Website offline".into());
    }
    if lead.instagram.map(|s| !s.trim().is_empty()).unwrap_or(false) {
        score += cfg.has_instagram;
        reasons.push("Instagram found".into());
    }
    if lead.phone.map(|s| !s.trim().is_empty()).unwrap_or(false) {
        score += cfg.has_phone;
        reasons.push("Phone available".into());
    }
    if lead.email.map(|s| !s.trim().is_empty()).unwrap_or(false) {
        score += cfg.has_email;
        reasons.push("Email found".into());
    }
    if lead.rating.map(|r| r >= cfg.rating_threshold).unwrap_or(false) {
        score += cfg.rating_gte;
        reasons.push("Strong Google reviews".into());
    }
    if lead.review_count.map(|c| c >= cfg.reviews_threshold).unwrap_or(false) {
        score += cfg.reviews_gte;
        reasons.push("Many reviews".into());
    }
    if matches!(lead.website_status, Some("active")) {
        score += cfg.active_bonus;
        reasons.push("Active business".into());
    }
    (score, reasons)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_no_website_lead_high() {
        let cfg = ScoreConfig::default();
        let (s, r) = compute_score(
            &cfg,
            &LeadView {
                website: None,
                website_status: None,
                instagram: Some("miamidental"),
                phone: Some("+1"),
                email: Some("a@b.com"),
                rating: Some(4.8),
                review_count: Some(281),
            },
        );
        assert!(s >= 75);
        assert!(r.contains(&"No website".to_string()));
    }

    #[test]
    fn scores_offline_website() {
        let cfg = ScoreConfig::default();
        let (s, _) = compute_score(
            &cfg,
            &LeadView {
                website: Some("https://x.com"),
                website_status: Some("unreachable"),
                instagram: None,
                phone: None,
                email: None,
                rating: None,
                review_count: None,
            },
        );
        assert_eq!(s, cfg.website_offline);
    }

    #[test]
    fn empty_lead_scores_zero() {
        let cfg = ScoreConfig::default();
        let (s, r) = compute_score(
            &cfg,
            &LeadView {
                website: Some("https://x.com"),
                website_status: Some("active"),
                instagram: None,
                phone: None,
                email: None,
                rating: None,
                review_count: None,
            },
        );
        assert_eq!(s, cfg.active_bonus);
        assert!(r.is_empty() || !r.contains(&"No website".to_string()));
    }
}
