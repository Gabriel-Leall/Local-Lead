use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchJob {
    pub id: i64,
    pub name: Option<String>,
    pub query: String,
    pub city: String,
    pub radius_meters: Option<f64>,
    pub status: String,
    pub result_count: i64,
    pub new_count: i64,
    pub error: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchJobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl SearchJobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}
