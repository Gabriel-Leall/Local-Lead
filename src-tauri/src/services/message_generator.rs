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
            label: "Sem site — primeiro contato",
            channel: "email",
            subject: "Uma ideia rápida para {name}",
            body: "Olá, equipe {name}!\n\nEncontrei {name}{category_part} pesquisando negócios locais{city_part}. Vi que vocês não têm site cadastrado, então alguns clientes podem ter dificuldade para encontrá-los.\n{rating_part}\nEu crio sites simples e rápidos para negócios locais. Gostariam de ver uma proposta gratuita, sem compromisso?\n\nAtenciosamente",
        },
        MessageTemplate {
            id: "generic",
            label: "Genérico — primeiro contato",
            channel: "email",
            subject: "Uma ideia rápida para {name}",
            body: "Olá, equipe {name}!\n\nEncontrei {name}{category_part}{city_part}.\n{rating_part}\nAjudo negócios locais a conquistar mais clientes com presença online simples. Faz sentido uma conversa rápida de 10 minutos?\n\nAtenciosamente",
        },
        MessageTemplate {
            id: "follow_up",
            label: "Retorno",
            channel: "email",
            subject: "Re: {name}",
            body: "Olá, equipe {name}!\n\nPassando para retomar minha mensagem anterior. Se melhorar a presença online é prioridade neste mês, posso compartilhar uma ideia rápida pensada para o negócio de vocês.\n\nAtenciosamente",
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
        .map(|c| format!(" em {c}"))
        .unwrap_or_default();
    let rating_part = match (ctx.rating, ctx.reviews) {
        (Some(r), Some(n)) => format!("Vi também a nota {r:.1} com {n} avaliações — parabéns."),
        (Some(r), None) => format!("Vi também a nota {r:.1} — parabéns."),
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
