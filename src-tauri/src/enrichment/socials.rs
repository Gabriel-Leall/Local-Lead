use regex::Regex;
use std::sync::OnceLock;

fn insta_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)instagram\.com/([A-Za-z0-9._]{1,30})").expect("insta regex")
    })
}

fn fb_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)facebook\.com/([A-Za-z0-9._-]{1,50})").expect("fb regex")
    })
}

const INSTA_SKIP: [&str; 6] = ["p", "reel", "reels", "stories", "explore", "share"];
const FB_SKIP: [&str; 5] = ["sharer", "share", "plugins", "tr", "login"];

pub fn normalize_instagram(raw: &str) -> Option<String> {
    let h = raw.trim().trim_start_matches('@').trim_matches('/').to_lowercase();
    if h.is_empty() || h.len() > 30 || h.contains('/') || h.contains(' ') {
        return None;
    }
    if INSTA_SKIP.contains(&h.as_str()) {
        return None;
    }
    Some(h)
}

pub fn extract_instagram(html: &str) -> Option<String> {
    for cap in insta_re().captures_iter(html) {
        if let Some(m) = cap.get(1) {
            if let Some(n) = normalize_instagram(m.as_str()) {
                return Some(n);
            }
        }
    }
    None
}

pub fn extract_facebook(html: &str) -> Option<String> {
    for cap in fb_re().captures_iter(html) {
        if let Some(m) = cap.get(1) {
            let v = m.as_str().trim_matches('/').to_string();
            if v.is_empty() || FB_SKIP.contains(&v.to_lowercase().as_str()) {
                continue;
            }
            return Some(format!("facebook.com/{v}"));
        }
    }
    None
}

pub fn extract_whatsapp(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    for marker in ["wa.me/", "api.whatsapp.com/send"] {
        if let Some(idx) = lower.find(marker) {
            let start = idx;
            let rest = &html[start..];
            let end = rest
                .find(|c: char| c == '"' || c == '\'' || c == '<' || c == ' ' || c == ')')
                .unwrap_or(rest.len().min(80));
            let url = rest[..end].to_string();
            if url.len() > marker.len() + 2 {
                return Some(url);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_instagram_handle() {
        let html = r#"<a href="https://instagram.com/miamidental/">ig</a>"#;
        assert_eq!(extract_instagram(html).as_deref(), Some("miamidental"));
    }

    #[test]
    fn normalizes_at_handle() {
        assert_eq!(normalize_instagram("@MiamiDental/").as_deref(), Some("miamidental"));
    }

    #[test]
    fn skips_instagram_paths() {
        assert_eq!(normalize_instagram("p"), None);
    }

    #[test]
    fn extracts_whatsapp_link() {
        let html = r#"<a href="https://wa.me/13050000000">wa</a>"#;
        assert!(extract_whatsapp(html).unwrap().contains("wa.me/13050000000"));
    }

    #[test]
    fn extracts_facebook() {
        let html = r#"<a href="https://facebook.com/miamidental">fb</a>"#;
        assert!(extract_facebook(html).is_some());
    }
}
