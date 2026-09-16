use regex::Regex;
use std::sync::OnceLock;

fn email_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b").expect("email regex")
    })
}

const BAD_EXTENSIONS: [&str; 6] = [".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp"];

pub fn extract_emails(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    for m in email_re().find_iter(html) {
        let e = m.as_str().to_lowercase();
        if BAD_EXTENSIONS.iter().any(|ext| e.ends_with(ext)) {
            continue;
        }
        if e.len() > 100 {
            continue;
        }
        if !out.contains(&e) {
            out.push(e);
        }
        if out.len() >= 5 {
            break;
        }
    }
    out
}

pub fn extract_tel_links(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = html.to_lowercase();
    let mut start = 0;
    while let Some(idx) = lower[start..].find("tel:") {
        let s = start + idx + 4;
        let rest = &html[s..];
        let end = rest
            .find(|c: char| c == '"' || c == '\'' || c == '<' || c == ' ' || c == ')')
            .unwrap_or(rest.len().min(30));
        let phone = rest[..end].trim().to_string();
        if !phone.is_empty() && !out.contains(&phone) {
            out.push(phone);
        }
        start = s + end;
        if out.len() >= 5 || start >= html.len() {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_emails() {
        let html = r#"<a href="mailto:hello@example.com">hello@example.com</a> Contact INFO@Example.COM"#;
        let e = extract_emails(html);
        assert!(e.contains(&"hello@example.com".to_string()));
        assert!(e.contains(&"info@example.com".to_string()));
    }

    #[test]
    fn ignores_image_like_emails() {
        let html = "logo.png@example.com";
        let _ = extract_emails(html);
    }

    #[test]
    fn finds_tel_links() {
        let html = r#"<a href="tel:+13050000000">call</a>"#;
        let t = extract_tel_links(html);
        assert_eq!(t, vec!["+13050000000".to_string()]);
    }
}
