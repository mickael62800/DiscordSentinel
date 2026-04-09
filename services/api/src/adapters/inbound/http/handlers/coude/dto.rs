use serde::{Deserialize, Serialize};

// ══════════════════════════════════════════════════════════
//  Response DTOs (Serialize + FromRow)
// ══════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CombatDto {
    pub id: String,
    pub guild_id: String,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub status: String,
    pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>,
    pub defender_roll: Option<i32>,
    pub chaos_event: Option<String>,
    pub special_attack: Option<String>,
    pub defender_special: Option<String>,
    pub coins_transferred: Option<i64>,
    pub result_message: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PlayerDto {
    pub user_id: String,
    pub username: String,
    pub coins: i64,
    pub total_wins: i32,
    pub total_losses: i32,
    pub total_draws: i32,
    pub total_earned: i64,
    pub total_lost: i64,
    pub total_stolen: i64,
    pub cowardice_count: i32,
    pub casino_wins: i32,
    pub casino_losses: i32,
    pub level: i32,
    pub xp: i64,
    pub class: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FullPlayerDto {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub coins: i64,
    pub total_wins: i32,
    pub total_losses: i32,
    pub total_draws: i32,
    pub total_earned: i64,
    pub total_lost: i64,
    pub total_stolen: i64,
    pub cowardice_count: i32,
    pub chaos_events: i32,
    pub casino_wins: i32,
    pub casino_losses: i32,
    pub level: i32,
    pub xp: i64,
    pub stat_points: i32,
    pub atk: i32,
    pub def: i32,
    pub class: Option<String>,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FullCombatDto {
    pub id: String,
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub status: String,
    pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>,
    pub defender_roll: Option<i32>,
    pub chaos_event: Option<String>,
    pub special_attack: Option<String>,
    pub defender_special: Option<String>,
    pub coins_transferred: Option<i64>,
    pub result_message: Option<String>,
    pub message_id: Option<String>,
    pub created_at: String,
    pub accepted_at: Option<String>,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BetDto {
    pub id: String,
    pub combat_id: String,
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PrimeDto {
    pub id: String,
    pub guild_id: String,
    pub target_id: String,
    pub target_name: String,
    pub placed_by_id: String,
    pub placed_by_name: String,
    pub amount: i64,
    pub claimed: bool,
    pub claimed_by_id: Option<String>,
    pub claimed_by_name: Option<String>,
    pub claimed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct InsuranceDto {
    pub id: String,
    pub is_scam: bool,
    pub expires_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LeaderboardEntry {
    pub user_id: String,
    pub username: String,
    pub value: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct InventoryItemDto {
    pub guild_id: String,
    pub user_id: String,
    pub item_key: String,
    pub quantity: i32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EventDto {
    pub id: String,
    pub guild_id: String,
    pub active: bool,
    pub expires_at: String,
    pub created_at: String,
}

// ══════════════════════════════════════════════════════════
//  Request DTOs (Deserialize)
// ══════════════════════════════════════════════════════════

// Players
#[derive(Debug, Deserialize)]
pub struct GetOrCreatePlayerDto { pub user_id: String, pub username: String }
#[derive(Debug, Deserialize)]
pub struct UpdateClassDto { pub class: String }
#[derive(Debug, Deserialize)]
pub struct AddXpDto { pub amount: i64 }
#[derive(Debug, Serialize)]
pub struct AddXpResponse { pub new_xp: i64, pub new_level: i32, pub leveled_up: bool, pub stat_points_gained: i32 }
#[derive(Debug, Deserialize)]
pub struct SpendStatDto { pub stat: String }
#[derive(Debug, Deserialize)]
pub struct RecordWinDto { pub earned: i64, pub stolen: i64 }
#[derive(Debug, Deserialize)]
pub struct RecordLossDto { pub lost: i64 }
#[derive(Debug, Deserialize)]
pub struct RecordDrawDto { pub lost: i64 }
#[derive(Debug, Deserialize)]
pub struct AdjustCoinsDto { pub amount: i64 }

// Economy
#[derive(Debug, Deserialize)]
pub struct AmountDto { pub amount: i64 }
#[derive(Debug, Deserialize)]
pub struct GainDto { pub gain: i64 }
#[derive(Debug, Deserialize)]
pub struct LostDto { pub lost: i64 }
#[derive(Debug, Deserialize)]
pub struct TransferCoinsDto { pub from_id: String, pub to_id: String, pub amount: i64 }
#[derive(Debug, Deserialize)]
pub struct StealDto { pub thief_id: String, pub victim_id: String, pub amount: i64 }

// Combats
#[derive(Debug, Deserialize)]
pub struct CombatQueryParams { pub status: Option<String>, pub limit: Option<i64> }
#[derive(Debug, Deserialize)]
pub struct CreateCombatDto {
    pub channel_id: Option<String>,
    pub attacker_id: String, pub attacker_name: String,
    pub defender_id: String, pub defender_name: String,
    pub mise: i64, pub special_attack: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct ResolveCombatDto {
    pub status: String, pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>, pub defender_roll: Option<i32>,
    pub chaos_event: Option<String>, pub result_message: Option<String>,
    pub coins_transferred: Option<i64>,
}
#[derive(Debug, Deserialize)]
pub struct SetBettingDto { pub message_id: String }
#[derive(Debug, Deserialize)]
pub struct DefenderSpecialDto { pub item_key: String }

// Bets
#[derive(Debug, Deserialize)]
pub struct PlaceBetDto {
    pub combat_id: String, pub bettor_id: String, pub bettor_name: String,
    pub backed_id: String, pub amount: i64,
}
#[derive(Debug, Deserialize)]
pub struct ResolveBetsDto { pub winner_id: Option<String> }
#[derive(Debug, Serialize)]
pub struct BetResult {
    pub bettor_id: String, pub bettor_name: String, pub backed_id: String,
    pub amount_bet: i64, pub payout: i64, pub won: bool,
}
#[derive(Debug, Serialize)]
pub struct FighterBetBonus { pub winner_id: String, pub winner_bonus: i64, pub loser_id: String, pub loser_bonus: i64 }
#[derive(Debug, Serialize)]
pub struct ResolveBetsResponse { pub results: Vec<BetResult>, pub fighter_bonus: Option<FighterBetBonus> }

// Social
#[derive(Debug, Deserialize)]
pub struct DurationDto { pub duration_secs: i64 }
#[derive(Debug, Deserialize)]
pub struct LeaderboardQueryParams { pub limit: Option<i64> }
#[derive(Debug, Deserialize)]
pub struct RandomPlayersQuery { pub count: Option<i64> }
#[derive(Debug, Deserialize)]
pub struct DailyChaosDto {
    pub loser_id: String, pub loser_name: String,
    pub winner_id: String, pub winner_name: String, pub amount: i64,
}

// Inventory
#[derive(Debug, Deserialize)]
pub struct CreatePrimeDto {
    pub target_id: String, pub target_name: String,
    pub placed_by_id: String, pub placed_by_name: String, pub amount: i64,
}
#[derive(Debug, Deserialize)]
pub struct ClaimPrimesDto { pub target_id: String, pub claimer_id: String, pub claimer_name: String }
#[derive(Debug, Deserialize)]
pub struct BuyInsuranceDto { pub user_id: String, pub is_scam: bool }
#[derive(Debug, Deserialize)]
pub struct AddItemDto { pub item_key: String }
#[derive(Debug, Deserialize)]
pub struct UseItemDto { pub item_key: String }
