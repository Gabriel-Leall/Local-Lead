pub fn normalize_domain(raw: Option<&str>) -> Option<String> {
    let r = raw?.trim();
    if r.is_empty() {
        return None;
    }
    let r = r.strip_prefix("https://").or_else(|| r.strip_prefix("http://")).unwrap_or(r);
    let r = r.strip_prefix("www.").unwrap_or(r);
    let host = r.split('/').next()?.split('?').next()?.trim().to_lowercase();
    if host.is_empty() || !host.contains('.') {
        return None;
    }
    Some(host)
}

pub fn normalize_phone(raw: Option<&str>) -> Option<String> {
    let r = raw?;
    let digits: String = r.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 7 {
        return None;
    }
    Some(format!("+{digits}"))
}

pub fn normalize_name(name: &str) -> String {
    name.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn coords_close(a_lat: Option<f64>, a_lng: Option<f64>, b_lat: Option<f64>, b_lng: Option<f64>) -> bool {
    match (a_lat, a_lng, b_lat, b_lng) {
        (Some(la1), Some(lo1), Some(la2), Some(lo2)) => (la1 - la2).abs() < 0.0005 && (lo1 - lo2).abs() < 0.0005,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_domains() {
        assert_eq!(normalize_domain(Some("https://www.Example.com/")).as_deref(), Some("example.com"));
        assert_eq!(normalize_domain(Some("http://example.com/a?b=1")).as_deref(), Some("example.com"));
        assert_eq!(normalize_domain(None), None);
        assert_eq!(normalize_domain(Some("notasite")), None);
    }

    #[test]
    fn normalizes_phones() {
        assert_eq!(normalize_phone(Some("+1 (305) 000-0000")).as_deref(), Some("+13050000000"));
        assert_eq!(normalize_phone(Some("abc")), None);
    }
}
