use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadContext {
    pub name: String,
    pub category: Option<String>,
    pub city: Option<String>,
    pub website: Option<String>,
    pub instagram: Option<String>,
    pub rating: Option<f64>,
    pub reviews: Option<i64>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

pub fn extract_city(address: Option<&str>) -> Option<String> {
    let a = address?.trim();
    if a.is_empty() {
        return None;
    }
    let first = a.split(',').next()?.trim();
    if first.is_empty() || first.len() > 60 {
        return None;
    }
    Some(first.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTemplate {
    pub id: &'static str,
    pub label: &'static str,
    pub channel: &'static str,
    pub subject: &'static str,
    pub body: &'static str,
}

pub fn builtin_templates() -> Vec<MessageTemplate> {
    vec![
        MessageTemplate {
            id: "no_website",
            label: "No website — first contact",
            channel: "email",
            subject: "Quick idea for {name}",
            body: "Hi {name} team,\n\nI found {name}{category_part} while looking at local businesses{city_part}. I noticed you don't have a website listed, so some customers may have trouble finding you.\n{rating_part}\nI build simple, fast websites for local businesses. Would you like a free mockup to review — no commitment?\n\nBest regards",
        },
        MessageTemplate {
            id: "generic",
            label: "Generic — first contact",
            channel: "email",
            subject: "Quick idea for {name}",
            body: "Hi {name} team,\n\nI found {name}{category_part}{city_part}.\n{rating_part}\nI help local businesses get more customers with a simple online presence. Would a quick 10-minute chat make sense?\n\nBest regards",
        },
        MessageTemplate {
            id: "follow_up",
            label: "Follow-up",
            channel: "email",
            subject: "Re: {name}",
            body: "Hi {name} team,\n\nJust following up on my previous message. If improving your online presence is a priority this month, I can share a quick idea tailored to your business.\n\nBest regards",
        },
    ]
}

pub fn render_template(t: &MessageTemplate, ctx: &LeadContext) -> (String, String) {
    let category_part = ctx
        .category
        .as_deref()
        .map(|c| format!(" ({c})"))
        .unwrap_or_default();
    let city_part = ctx
        .city
        .as_deref()
        .map(|c| format!(" in {c}"))
        .unwrap_or_default();
    let rating_part = match (ctx.rating, ctx.reviews) {
        (Some(r), Some(n)) => format!("I also saw your {r:.1} rating with {n} reviews — nice work."),
        (Some(r), None) => format!("I also saw your {r:.1} rating — nice work."),
        _ => String::new(),
    };
    let subject = t
        .subject
        .replace("{name}", &ctx.name)
        .replace("{category_part}", &category_part)
        .replace("{city_part}", &city_part);
    let body = t
        .body
        .replace("{name}", &ctx.name)
        .replace("{category_part}", &category_part)
        .replace("{city_part}", &city_part)
        .replace("{rating_part}", &rating_part)
        .replace("{instagram}", ctx.instagram.as_deref().unwrap_or(""))
        .replace("{website}", ctx.website.as_deref().unwrap_or(""));
    let body = body
        .lines()
        .filter(|l| !l.trim().is_empty() || true)
        .collect::<Vec<_>>()
        .join("\n")
        .replace("\n\n\n", "\n\n")
        .trim()
        .to_string();
    (subject, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> LeadContext {
        LeadContext {
            name: "Miami Dental".into(),
            category: Some("dentist".into()),
            city: Some("Miami".into()),
            website: None,
            instagram: Some("miamidental".into()),
            rating: Some(4.8),
            reviews: Some(281),
            phone: None,
            email: None,
        }
    }

    #[test]
    fn renders_without_inventing() {
        let t = builtin_templates().into_iter().find(|t| t.id == "generic").unwrap();
        let (subj, body) = render_template(&t, &ctx());
        assert!(subj.contains("Miami Dental"));
        assert!(body.contains("4.8"));
        assert!(!body.contains("{rating_part}"));
        assert!(!body.to_lowercase().contains("owner"));
    }

    #[test]
    fn omits_rating_when_missing() {
        let mut c = ctx();
        c.rating = None;
        c.reviews = None;
        let t = builtin_templates().into_iter().find(|t| t.id == "no_website").unwrap();
        let (_, body) = render_template(&t, &c);
        assert!(!body.contains("rating"));
        assert!(!body.contains('{'));
    }

    #[test]
    fn extracts_city() {
        assert_eq!(extract_city(Some("Miami, FL")).as_deref(), Some("Miami"));
        assert_eq!(extract_city(None), None);
        assert_eq!(extract_city(Some("")), None);
    }
}
