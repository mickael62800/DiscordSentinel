//! Tous les DTOs HTTP (requête et réponse) du module Coup de Coude.
//!
//! Chaque DTO de réponse dérive `Serialize` et fournit une conversion
//! `From<domain_entity>` pour que les handlers restent des mappers triviaux.

use sentinel_core::domain::entities::coude::bet::Bet;
use sentinel_core::domain::entities::coude::bet::BetResolutionPlan;
use sentinel_core::domain::entities::coude::bet::FighterBetBonus as CoudeFighterBetBonus;
use sentinel_core::domain::entities::coude::combat::Combat;
use sentinel_core::domain::entities::coude::inventory::Insurance;
use sentinel_core::domain::entities::coude::inventory::InventoryItem;
use sentinel_core::domain::entities::coude::inventory::Prime;
use sentinel_core::domain::entities::coude::player::Player;
use sentinel_core::domain::entities::coude::player::XpProgress;
use sentinel_core::domain::entities::coude::social::Event;
use sentinel_core::domain::entities::coude::social::LeaderboardEntry;
use sentinel_core::domain::entities::coude::social::Season;
use sentinel_core::domain::entities::system::discord_ids::GuildId;
use sentinel_core::domain::entities::system::discord_ids::MessageId;
use sentinel_core::domain::entities::system::discord_ids::UserId;
use serde::Deserialize;
use serde::Serialize;
// ══════════════════════════════════════════════════════════════════════
// ── Player DTOs ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct PlayerDto {
    pub user_id: UserId,
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

impl From<&Player> for PlayerDto {
    fn from(p: &Player) -> Self {
        Self {
            user_id: p.user_id.clone(),
            username: p.username.clone(),
            coins: p.coins,
            total_wins: p.total_wins,
            total_losses: p.total_losses,
            total_draws: p.total_draws,
            total_earned: p.total_earned,
            total_lost: p.total_lost,
            total_stolen: p.total_stolen,
            cowardice_count: p.cowardice_count,
            casino_wins: p.casino_wins,
            casino_losses: p.casino_losses,
            level: p.level,
            xp: p.xp,
            class: p.class.as_ref().map(|c| c.as_str().to_string()),
            title: p.title.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FullPlayerDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
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
    pub hp_current: i32,
    pub hp_max: i32,
    pub hp_last_regen: Option<String>,
    pub class_changed_at: Option<String>,
    pub repos_last_used: Option<String>,
    pub season: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Player> for FullPlayerDto {
    fn from(p: Player) -> Self {
        Self {
            guild_id: p.guild_id,
            user_id: p.user_id,
            username: p.username,
            coins: p.coins,
            total_wins: p.total_wins,
            total_losses: p.total_losses,
            total_draws: p.total_draws,
            total_earned: p.total_earned,
            total_lost: p.total_lost,
            total_stolen: p.total_stolen,
            cowardice_count: p.cowardice_count,
            chaos_events: p.chaos_events,
            casino_wins: p.casino_wins,
            casino_losses: p.casino_losses,
            level: p.level,
            xp: p.xp,
            stat_points: p.stat_points,
            atk: p.atk,
            def: p.def,
            class: p.class.map(|c| c.as_str().to_string()),
            title: p.title,
            hp_current: p.hp_current,
            hp_max: p.hp_max,
            hp_last_regen: p.hp_last_regen.map(|d| d.to_rfc3339()),
            class_changed_at: p.class_changed_at.map(|d| d.to_rfc3339()),
            repos_last_used: p.repos_last_used.map(|d| d.to_rfc3339()),
            season: p.season,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GetOrCreatePlayerDto {
    pub user_id: UserId,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateClassDto {
    pub class: String,
}

#[derive(Debug, Deserialize)]
pub struct AddXpDto {
    pub amount: i64,
}

#[derive(Debug, Serialize)]
pub struct AddXpResponse {
    pub new_xp: i64,
    pub new_level: i32,
    pub leveled_up: bool,
    pub stat_points_gained: i32,
}

impl From<XpProgress> for AddXpResponse {
    fn from(p: XpProgress) -> Self {
        Self {
            new_xp: p.new_xp,
            new_level: p.new_level,
            leveled_up: p.leveled_up,
            stat_points_gained: p.stat_points_gained,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SpendStatDto {
    pub stat: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetStatsDto {
    pub cost: i64,
}

#[derive(Debug, Deserialize)]
pub struct RecordWinDto {
    pub earned: i64,
    pub stolen: i64,
}

#[derive(Debug, Deserialize)]
pub struct RecordLossDto {
    pub lost: i64,
}

#[derive(Debug, Deserialize)]
pub struct RecordDrawDto {
    pub lost: i64,
}

#[derive(Debug, Deserialize)]
pub struct AmountDto {
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct AdjustCoinsDto {
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateHpDto {
    pub hp_current: i32,
    pub hp_max: i32,
}

#[derive(Debug, Deserialize)]
pub struct RandomPlayersQuery {
    pub count: Option<i64>,
}

// ══════════════════════════════════════════════════════════════════════
// ── Combat DTOs ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct CombatDto {
    pub id: String,
    pub guild_id: GuildId,
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

impl From<&Combat> for CombatDto {
    fn from(c: &Combat) -> Self {
        Self {
            id: c.id.to_string(),
            guild_id: c.guild_id.clone(),
            attacker_id: c.attacker_id.clone(),
            attacker_name: c.attacker_name.clone(),
            defender_id: c.defender_id.clone(),
            defender_name: c.defender_name.clone(),
            mise: c.mise,
            status: c.status.clone(),
            winner_id: c.winner_id.clone(),
            attacker_roll: c.attacker_roll,
            defender_roll: c.defender_roll,
            chaos_event: c.chaos_event.clone(),
            special_attack: c.special_attack.clone(),
            defender_special: c.defender_special.clone(),
            coins_transferred: c.coins_transferred,
            result_message: c.result_message.clone(),
            created_at: c.created_at.to_rfc3339(),
            resolved_at: c.resolved_at.map(|d| d.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FullCombatDto {
    pub id: String,
    pub guild_id: GuildId,
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

impl From<Combat> for FullCombatDto {
    fn from(c: Combat) -> Self {
        Self {
            id: c.id.to_string(),
            guild_id: c.guild_id,
            channel_id: c.channel_id,
            attacker_id: c.attacker_id,
            attacker_name: c.attacker_name,
            defender_id: c.defender_id,
            defender_name: c.defender_name,
            mise: c.mise,
            status: c.status,
            winner_id: c.winner_id,
            attacker_roll: c.attacker_roll,
            defender_roll: c.defender_roll,
            chaos_event: c.chaos_event,
            special_attack: c.special_attack,
            defender_special: c.defender_special,
            coins_transferred: c.coins_transferred,
            result_message: c.result_message,
            message_id: c.message_id,
            created_at: c.created_at.to_rfc3339(),
            accepted_at: c.accepted_at.map(|d| d.to_rfc3339()),
            resolved_at: c.resolved_at.map(|d| d.to_rfc3339()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CombatQueryParams {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCombatDto {
    pub channel_id: Option<String>,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub special_attack: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveCombatDto {
    pub status: String,
    pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>,
    pub defender_roll: Option<i32>,
    pub chaos_event: Option<String>,
    pub result_message: Option<String>,
    pub coins_transferred: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SetBettingDto {
    pub message_id: MessageId,
}

#[derive(Debug, Deserialize)]
pub struct DefenderSpecialDto {
    pub item_key: String,
}

// ══════════════════════════════════════════════════════════════════════
// ── Bet DTOs ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct BetDto {
    pub id: String,
    pub combat_id: String,
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount: i64,
}

impl From<&Bet> for BetDto {
    fn from(b: &Bet) -> Self {
        Self {
            id: b.id.to_string(),
            combat_id: b.combat_id.to_string(),
            bettor_id: b.bettor_id.clone(),
            bettor_name: b.bettor_name.clone(),
            backed_id: b.backed_id.clone(),
            amount: b.amount,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PlaceBetDto {
    pub combat_id: String,
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount: i64,
}

/// Reponse du POST /api/coude/{guild_id}/bets apres la Migration #7 :
/// expose les TauntEvents declenches (faillite parieur) pour que le bot
/// les dispatche en un seul aller-retour (meme pattern que `/donner`).
#[derive(Debug, Serialize)]
pub struct PlaceBetResponse {
    pub taunt_events: Vec<super::taunts::TauntEventDto>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveBetsDto {
    pub winner_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BetResult {
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount_bet: i64,
    pub payout: i64,
    pub won: bool,
}

#[derive(Debug, Serialize)]
pub struct FighterBetBonus {
    pub winner_id: String,
    pub winner_bonus: i64,
    pub loser_id: String,
    pub loser_bonus: i64,
    pub total_pot: i64,
}

impl From<CoudeFighterBetBonus> for FighterBetBonus {
    fn from(b: CoudeFighterBetBonus) -> Self {
        Self {
            winner_id: b.winner_id,
            winner_bonus: b.winner_bonus,
            loser_id: b.loser_id,
            loser_bonus: b.loser_bonus,
            total_pot: b.total_pot,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ResolveBetsResponse {
    pub results: Vec<BetResult>,
    pub fighter_bonus: Option<FighterBetBonus>,
    /// Migration #7 : TauntEvents declenches par l'application des paris
    /// (jackpots parieurs gagnants, bonus combattants). Vide par defaut,
    /// rempli par le handler apres mapping depuis `ResolveBetsOutcome`.
    #[serde(default)]
    pub taunt_events: Vec<super::taunts::TauntEventDto>,
}

impl From<BetResolutionPlan> for ResolveBetsResponse {
    fn from(plan: BetResolutionPlan) -> Self {
        Self {
            results: plan
                .payouts
                .into_iter()
                .map(|p| BetResult {
                    bettor_id: p.bettor_id,
                    bettor_name: p.bettor_name,
                    backed_id: p.backed_id,
                    amount_bet: p.amount_bet,
                    payout: p.payout,
                    won: p.won,
                })
                .collect(),
            fighter_bonus: plan.fighter_bonus.map(FighterBetBonus::from),
            taunt_events: Vec::new(),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// ── Economy DTOs ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct GainDto {
    pub gain: i64,
}

#[derive(Debug, Deserialize)]
pub struct LostDto {
    pub lost: i64,
}

#[derive(Debug, Deserialize)]
pub struct TransferCoinsDto {
    pub from_id: String,
    pub to_id: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct StealDto {
    pub thief_id: String,
    pub victim_id: String,
    pub amount: i64,
}

// ══════════════════════════════════════════════════════════════════════
// ── Inventory / Primes / Insurance DTOs ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct InventoryItemDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub item_key: String,
    pub quantity: i32,
}

impl From<InventoryItem> for InventoryItemDto {
    fn from(i: InventoryItem) -> Self {
        Self {
            guild_id: i.guild_id,
            user_id: i.user_id,
            item_key: i.item_key,
            quantity: i.quantity,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AddItemDto {
    pub item_key: String,
}

#[derive(Debug, Deserialize)]
pub struct UseItemDto {
    pub item_key: String,
}

#[derive(Debug, Serialize)]
pub struct PrimeDto {
    pub id: String,
    pub guild_id: GuildId,
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

impl From<Prime> for PrimeDto {
    fn from(p: Prime) -> Self {
        Self {
            id: p.id.to_string(),
            guild_id: p.guild_id,
            target_id: p.target_id,
            target_name: p.target_name,
            placed_by_id: p.placed_by_id,
            placed_by_name: p.placed_by_name,
            amount: p.amount,
            claimed: p.claimed,
            claimed_by_id: p.claimed_by_id,
            claimed_by_name: p.claimed_by_name,
            claimed_at: p.claimed_at.map(|d| d.to_rfc3339()),
            created_at: p.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreatePrimeDto {
    pub target_id: String,
    pub target_name: String,
    pub placed_by_id: String,
    pub placed_by_name: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct ClaimPrimesDto {
    pub target_id: String,
    pub claimer_id: String,
    pub claimer_name: String,
}

#[derive(Debug, Serialize)]
pub struct InsuranceDto {
    pub id: String,
    pub is_scam: bool,
    pub expires_at: String,
}

impl From<Insurance> for InsuranceDto {
    fn from(i: Insurance) -> Self {
        Self {
            id: i.id.to_string(),
            is_scam: i.is_scam,
            expires_at: i.expires_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct BuyInsuranceDto {
    pub user_id: UserId,
    pub is_scam: bool,
    /// Duree en secondes. 0 ou absent = defaut 3600 (1h) pour retrocompat.
    #[serde(default)]
    pub duration_seconds: i64,
}

// ══════════════════════════════════════════════════════════════════════
// ── Social DTOs ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct LeaderboardEntryDto {
    pub user_id: UserId,
    pub username: String,
    pub value: i64,
}

impl From<LeaderboardEntry> for LeaderboardEntryDto {
    fn from(e: LeaderboardEntry) -> Self {
        Self {
            user_id: e.user_id,
            username: e.username,
            value: e.value,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LeaderboardQueryParams {
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DurationDto {
    pub duration_secs: i64,
}

#[derive(Debug, Deserialize)]
pub struct DailyChaosDto {
    pub loser_id: String,
    pub loser_name: String,
    pub winner_id: String,
    pub winner_name: String,
    pub amount: i64,
}

#[derive(Debug, Serialize)]
pub struct EventDto {
    pub id: String,
    pub guild_id: GuildId,
    pub active: bool,
    pub expires_at: String,
    pub created_at: String,
}

impl From<Event> for EventDto {
    fn from(e: Event) -> Self {
        Self {
            id: e.id.to_string(),
            guild_id: e.guild_id,
            active: e.active,
            expires_at: e.expires_at.to_rfc3339(),
            created_at: e.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CurrentSeasonDto {
    pub season_number: i32,
    pub started_at: String,
    pub ends_at: String,
    pub days_remaining: i64,
}

impl From<Season> for CurrentSeasonDto {
    fn from(s: Season) -> Self {
        Self {
            season_number: s.season_number,
            started_at: s.started_at.to_rfc3339(),
            ends_at: s.ends_at.to_rfc3339(),
            days_remaining: s.days_remaining,
        }
    }
}

#[cfg(test)]
#[path = "tests/dto.rs"]
mod tests;
