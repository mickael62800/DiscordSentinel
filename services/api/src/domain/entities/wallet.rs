use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub coins: i64,
    pub total_earned: i64,
    pub total_spent: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub amount: i64,
    pub balance_after: i64,
    pub source: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}
