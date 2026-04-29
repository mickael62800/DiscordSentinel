use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

// ══════════════════════════════════════════════════════════════════════
// ── Inventaire ──
// ══════════════════════════════════════════════════════════════════════

/// Une ligne d'inventaire d'un joueur (clé d'item + quantité).
#[derive(Debug, Clone)]
pub struct InventoryItem {
    pub guild_id: String,
    pub user_id: String,
    pub item_key: String,
    pub quantity: i32,
}

// ══════════════════════════════════════════════════════════════════════
// ── Primes (bounties) ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Prime {
    pub id: Uuid,
    pub guild_id: String,
    pub target_id: String,
    pub target_name: String,
    pub placed_by_id: String,
    pub placed_by_name: String,
    pub amount: i64,
    pub claimed: bool,
    pub claimed_by_id: Option<String>,
    pub claimed_by_name: Option<String>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewCoudePrime {
    pub guild_id: String,
    pub target_id: String,
    pub target_name: String,
    pub placed_by_id: String,
    pub placed_by_name: String,
    pub amount: i64,
}

// ══════════════════════════════════════════════════════════════════════
// ── Assurances ──
// ══════════════════════════════════════════════════════════════════════

/// Projection légère : les handlers n'exposent que ce qui est strictement
/// nécessaire côté bot (id + is_scam + expires_at).
#[derive(Debug, Clone)]
pub struct Insurance {
    pub id: Uuid,
    pub is_scam: bool,
    pub expires_at: DateTime<Utc>,
}
