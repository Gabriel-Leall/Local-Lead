use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LeadStatus {
    New,
    Qualified,
    Skipped,
}

impl LeadStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Qualified => "qualified",
            Self::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "qualified" => Self::Qualified,
            "skipped" => Self::Skipped,
            _ => Self::New,
        }
    }

    pub fn all() -> Vec<String> {
        vec!["new".into(), "qualified".into(), "skipped".into()]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lead {
    pub id: i64,
    pub canonical_name: String,
    pub category: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub rating: Option<f64>,
    pub review_count: Option<i64>,
    pub email: Option<String>,
    pub instagram: Option<String>,
    pub facebook: Option<String>,
    pub whatsapp: Option<String>,
    pub website_status: Option<String>,
    pub score: i64,
    pub score_reasons: Option<String>,
    pub lead_status: String,
    pub follow_up_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub const PIPELINE: [&str; 13] = [
    "new",
    "qualified",
    "message_ready",
    "contacted",
    "replied",
    "interested",
    "meeting",
    "won",
    "lost",
    "skipped",
    "do_not_contact",
    "message_sent",
    "responded",
];

pub fn is_valid_status(s: &str) -> bool {
    PIPELINE.contains(&s) || s == "message_sent" || s == "responded"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPlace {
    pub external_id: String,
    pub name: String,
    pub category: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub address: Option<String>,
    pub provider: String,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub rating: Option<f64>,
    pub review_count: Option<i64>,
}
