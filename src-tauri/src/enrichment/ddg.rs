use crate::error::AppError;
use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default)]
pub struct DdgResult {
    pub title: String,
    pub url: String,
    /// Trecho do resultado; guardado para uso futuro (ex: contexto de mensagem).
    #[allow(dead_code)]
    pub snippet: String,
}

fn link_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)<a[^>]+class="result__a"[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#)
            .expect("ddg link regex")
    })
}

fn snippet_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)class="result__snippet"[^>]*>(.*?)</"#).expect("ddg snippet regex")
    })
}

fn tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<[^>]+>").expect("tag regex"))
}

fn clean_html(s: &str) -> String {
    tag_re()
        .replace_all(s, "")
        .replace("&amp;", "&")
        .replace("&#x27;", "'")
        .replace("&quot;", "\"")
        .trim()
        .to_string()
}

/// Desembrulha redirects //duckduckgo.com/l/?uddg=<url-encoded>.
fn unwrap_url(href: &str) -> String {
    if let Some(idx) = href.find("uddg=") {
        let enc = &href[idx + 5..];
        let end = enc.find('&').unwrap_or(enc.len());
        return urlencoding_decode(&enc[..end]);
    }
    href.to_string()
}

fn urlencoding_decode(s: &str) -> String {
    let mut out = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                    out.push((h * 16 + l) as char);
                    i += 3;
                    continue;
                }
                out.push('%');
                i += 1;
            }
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b => {
                out.push(b as char);
                i += 1;
            }
        }
    }
    out
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub fn parse_results(html: &str) -> Vec<DdgResult> {
    let links: Vec<(String, String)> = link_re()
        .captures_iter(html)
        .map(|c| (unwrap_url(&c[1]), clean_html(&c[2])))
        .collect();
    let snippets: Vec<String> = snippet_re()
        .captures_iter(html)
        .map(|c| clean_html(&c[1]))
        .collect();
    links
        .into_iter()
        .enumerate()
        .map(|(i, (url, title))| DdgResult {
            title,
            url,
            snippet: snippets.get(i).cloned().unwrap_or_default(),
        })
        .filter(|r| !r.url.is_empty() && (r.url.starts_with("http") || r.url.starts_with("www.")))
        .take(8)
        .collect()
}

const SOCIAL_HOSTS: [&str; 7] = [
    "instagram.com",
    "facebook.com",
    "linkedin.com",
    "youtube.com",
    "twitter.com",
    "tiktok.com",
    "google.com",
];

pub fn is_social_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    SOCIAL_HOSTS.iter().any(|h| lower.contains(h))
}

pub async fn ddg_search(query: &str) -> Result<Vec<DdgResult>, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36")
        .build()
        .map_err(|e| AppError::Network(e.to_string()))?;
    let resp = client
        .get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)])
        .send()
        .await?;
    if status_blocked(resp.status()) {
        return Err(AppError::RateLimit);
    }
    if !resp.status().is_success() {
        return Err(AppError::Provider(format!("busca web http {}", resp.status())));
    }
    let html = resp.text().await.map_err(|e| AppError::Parse(e.to_string()))?;
    Ok(parse_results(&html))
}

fn status_blocked(s: reqwest::StatusCode) -> bool {
    s.as_u16() == 429 || s.as_u16() == 202
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
        <a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fararunaodontologia.com%2F&amp;rut=abc">Araruna Odontologia</a>
        <a class="result__snippet" href="/l/?uddg=x">Melhor clínica de Picos</a>
        <a rel="nofollow" class="result__a" href="https://instagram.com/ararunaodonto">Instagram</a>
    "#;

    #[test]
    fn parses_links_and_snippets() {
        let r = parse_results(FIXTURE);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].url, "https://ararunaodontologia.com/");
        assert_eq!(r[0].title, "Araruna Odontologia");
        assert_eq!(r[0].snippet, "Melhor clínica de Picos");
    }

    #[test]
    fn detects_social_urls() {
        assert!(is_social_url("https://instagram.com/x"));
        assert!(!is_social_url("https://clinica.com.br"));
    }
}
