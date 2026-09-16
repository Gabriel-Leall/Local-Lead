use super::contacts::{extract_emails, extract_tel_links};
use super::socials::{extract_facebook, extract_instagram, extract_whatsapp};
use crate::error::AppError;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WebsiteStatus {
    Unknown,
    None,
    Active,
    Unreachable,
    Redirect,
    Parked,
    SocialOnly,
}

impl WebsiteStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::None => "none",
            Self::Active => "active",
            Self::Unreachable => "unreachable",
            Self::Redirect => "redirect",
            Self::Parked => "parked",
            Self::SocialOnly => "social_only",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct WebsiteCrawlResult {
    pub status: Option<String>,
    pub email: Option<String>,
    pub instagram: Option<String>,
    pub facebook: Option<String>,
    pub whatsapp: Option<String>,
    pub phone: Option<String>,
}

const PATHS: [&str; 5] = ["/", "/contact", "/contact-us", "/about", "/about-us"];
const PARKED_MARKERS: [&str; 7] = [
    "buy this domain",
    "domain is parked",
    "parked domain",
    "godaddy",
    "sedoparking",
    "hugedomains",
    "this domain may be for sale",
];

pub fn detect_parked(html: &str) -> bool {
    let lower = html.to_lowercase();
    PARKED_MARKERS.iter().any(|m| lower.contains(m))
}

pub fn normalize_website(input: &str) -> Option<String> {
    let t = input.trim();
    if t.is_empty() {
        return None;
    }
    let t = t.strip_prefix("https://").or_else(|| t.strip_prefix("http://")).unwrap_or(t);
    let t = t.strip_prefix("www.").unwrap_or(t);
    let host = t.split('/').next()?.split('?').next()?.trim();
    if host.is_empty() || !host.contains('.') {
        return None;
    }
    Some(host.to_lowercase())
}

fn join_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    if path == "/" {
        return base.to_string();
    }
    format!("{base}{path}")
}

fn ensure_scheme(raw: &str) -> String {
    if raw.starts_with("http://") || raw.starts_with("https://") {
        raw.to_string()
    } else {
        format!("https://{raw}")
    }
}

pub async fn crawl_website(raw_website: &str) -> Result<WebsiteCrawlResult, AppError> {
    let norm = normalize_website(raw_website);
    if norm.is_none() {
        return Ok(WebsiteCrawlResult { status: Some(WebsiteStatus::None.as_str().into()), ..Default::default() });
    }
    let base = ensure_scheme(raw_website.trim());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .user_agent("LocalLeadProspector/0.1 (+local-first)")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| AppError::Network(e.to_string()))?;

    let mut combined = String::new();
    let mut ok_pages = 0;
    let mut final_host: Option<String> = None;
    let mut any_redirect = false;

    for path in PATHS {
        let url = join_url(&base, path);
        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                if e.is_timeout() || e.is_connect() {
                    continue;
                }
                continue;
            }
        };
        if resp.status().is_redirection() {
            any_redirect = true;
            continue;
        }
        if !resp.status().is_success() {
            continue;
        }
        let url_host = resp.url().host_str().map(|s| s.to_string());
        if final_host.is_none() {
            final_host = url_host.clone();
        }
        if let (Some(fh), Some(orig)) = (url_host.as_deref(), norm.as_deref()) {
            let fh = fh.strip_prefix("www.").unwrap_or(fh);
            if fh.to_lowercase() != orig {
                any_redirect = true;
            }
        }
        match resp.text().await {
            Ok(t) => {
                ok_pages += 1;
                combined.push_str(&t);
                combined.push('\n');
                if combined.len() > 600_000 {
                    break;
                }
                if ok_pages >= 3 && !combined.is_empty() {
                    break;
                }
            }
            Err(_) => continue,
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }

    if ok_pages == 0 {
        return Ok(WebsiteCrawlResult { status: Some(WebsiteStatus::Unreachable.as_str().into()), ..Default::default() });
    }
    if detect_parked(&combined) {
        return Ok(WebsiteCrawlResult { status: Some(WebsiteStatus::Parked.as_str().into()), ..Default::default() });
    }

    let emails = extract_emails(&combined);
    let instagram = extract_instagram(&combined);
    let facebook = extract_facebook(&combined);
    let whatsapp = extract_whatsapp(&combined);
    let tels = extract_tel_links(&combined);

    let status = if any_redirect {
        WebsiteStatus::Redirect.as_str()
    } else {
        WebsiteStatus::Active.as_str()
    };

    Ok(WebsiteCrawlResult {
        status: Some(status.into()),
        email: emails.into_iter().next(),
        instagram,
        facebook,
        whatsapp,
        phone: tels.into_iter().next(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_hosts() {
        assert_eq!(normalize_website("https://www.Example.com/").as_deref(), Some("example.com"));
        assert_eq!(normalize_website("http://example.com/contact").as_deref(), Some("example.com"));
        assert_eq!(normalize_website("not a site"), None);
    }

    #[test]
    fn detects_parked_pages() {
        assert!(detect_parked("This domain is parked by GoDaddy"));
        assert!(!detect_parked("Welcome to Miami Dental, book your visit"));
    }

    #[test]
    fn none_for_empty_website() {
        assert_eq!(normalize_website(""), None);
    }
}
