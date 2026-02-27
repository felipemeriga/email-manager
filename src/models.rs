use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSummary {
    pub id: String,
    pub subject: String,
    pub sender: String,
    pub sender_email: String,
    pub date: DateTime<Utc>,
    pub snippet: String,
    pub is_read: bool,
    pub labels: Vec<String>,
    pub importance_score: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportanceScore {
    Low = 1,
    Normal = 2,
    High = 3,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    #[serde(default = "default_min_score")]
    pub min_score: u8,
}

fn default_min_score() -> u8 {
    1
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkDeleteRequest {
    pub ids: Vec<String>,
}
