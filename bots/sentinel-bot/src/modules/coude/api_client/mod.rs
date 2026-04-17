#![allow(dead_code)]
//! Client API du coude-bot.
//!
//! Phase 7A : CoudePlayerService (6 RPCs hot path joueurs) migre.
//! Phase 7A.opt F.1 : 5 services supplementaires migres (combats, bets,
//! economy, inventory, social). ~80% du wrapper passe maintenant par gRPC.
//!
//! Restent en HTTP (pas d'equivalent dans les use cases exposes en proto) :
//! - methodes player "legacy" : spend_stat_point, reset_stats, record_win/
//!   loss/draw, increment_cowardice/chaos, record_coins_earned/lost, repos
//! - get_all_guild_ids, get_random_players (admin queries rares)
//!
//! Surface publique (types + signatures) inchangee : handlers et commandes
//! du bot n'ont pas a etre touches.

use std::sync::Arc;

use serde::Deserialize;
use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::grpc_client::SentinelGrpcClient;

use sentinel_proto::coude::v1 as proto_coude;

// ══════════════════════════════════════════════════════════════════════
// ── Response DTOs (preservation de la surface publique) ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Player {
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
    #[serde(default)]
    pub class_changed_at: Option<String>,
    #[serde(default)]
    pub hp_current: Option<i32>,
    #[serde(default)]
    pub hp_max: Option<i32>,
    #[serde(default)]
    pub hp_last_regen: Option<String>,
    #[serde(default)]
    pub repos_last_used: Option<String>,
    #[serde(default)]
    pub season: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Combat {
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

/// Phase 7 : donnees d'embed pretes a poster, retournees par
/// `resolve_combat_now`. Le bot les transforme en `CreateEmbed` sans aucune
/// logique metier supplementaire.
#[derive(Debug, Clone)]
pub struct ResolvedCombatEmbed {
    pub title: String,
    pub description: String,
    pub color: u32,
    pub fields: Vec<ResolvedCombatEmbedField>,
    /// Phase 9 Part D : railleries a poster apres l'embed.
    pub taunt_events: Vec<TauntEvent>,
}

#[derive(Debug, Clone)]
pub struct ResolvedCombatEmbedField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

/// Phase 9 Part D — Raillerie cuisinee cote API, pretes a poster tel quel.
/// Le bot ne fait que poster + renommer.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TauntEvent {
    pub channel_id: String,
    pub target_user_id: String,
    pub message: String,
    pub nickname_suffix: String,
    pub streak_kind: String,
    pub streak_value: i32,
}

// ── Phase 10 : braquage ──

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HeistResult {
    pub success: bool,
    pub chance_percent: u32,
    pub cashbox_total_before: i64,
    pub amount_stolen: i64,
    pub tools_consumed: Vec<String>,
    pub prison_released_at: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HeistCooldown {
    pub ready: bool,
    pub next_attempt_at: Option<String>,
    pub last_success: Option<bool>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PrisonStatus {
    pub in_prison: bool,
    pub released_at: Option<String>,
    pub reason: Option<String>,
}

fn taunt_event_from_proto(e: proto_coude::TauntEvent) -> TauntEvent {
    TauntEvent {
        channel_id: e.channel_id,
        target_user_id: e.target_user_id,
        message: e.message,
        nickname_suffix: e.nickname_suffix,
        streak_kind: e.streak_kind,
        streak_value: e.streak_value,
    }
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct WalletTransaction {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    pub amount: i64,
    pub balance_after: i64,
    pub source: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Prime {
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

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct InventoryItem {
    pub guild_id: String,
    pub user_id: String,
    pub item_key: String,
    pub quantity: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ServerEvent {
    pub id: String,
    pub guild_id: String,
    #[serde(default)]
    pub event_type: String,
    pub active: bool,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct LeaderboardEntry {
    pub user_id: String,
    pub username: String,
    pub value: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Bet {
    pub id: String,
    pub combat_id: String,
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct BetResult {
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount_bet: i64,
    pub payout: i64,
    pub won: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct FighterBetBonus {
    pub winner_id: String,
    pub winner_bonus: i64,
    pub loser_id: String,
    pub loser_bonus: i64,
    #[serde(default)]
    pub total_pot: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Insurance {
    pub id: String,
    pub is_scam: bool,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct XpResult {
    pub new_xp: i64,
    pub new_level: i32,
    pub leveled_up: bool,
    pub stat_points_gained: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CurrentSeason {
    pub season_number: i32,
    pub started_at: String,
    pub ends_at: String,
    pub days_remaining: i64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Cashbox {
    pub guild_id: String,
    pub balance: i64,
    pub total_collected: i64,
    pub total_redistributed: i64,
    pub last_redistribution_at: Option<String>,
}

/// Source d'un depot dans la caisse. Miroir cote bot de l'enum proto,
/// pour que les commandes n'aient pas a importer les types generes.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum CashboxDepositSource {
    ShopPurchase,
    InsurancePurchase,
    ProtectionPurchase,
    BoostPurchase,
    ClassChangeCost,
    ResetStatsCost,
    DonationTax,
    CowardicePenalty,
    BetCommission,
}

impl CashboxDepositSource {
    fn as_proto(self) -> proto_coude::CashboxDepositSource {
        use proto_coude::CashboxDepositSource as P;
        match self {
            Self::ShopPurchase => P::CashboxSourceShopPurchase,
            Self::InsurancePurchase => P::CashboxSourceInsurancePurchase,
            Self::ProtectionPurchase => P::CashboxSourceProtectionPurchase,
            Self::BoostPurchase => P::CashboxSourceBoostPurchase,
            Self::ClassChangeCost => P::CashboxSourceClassChangeCost,
            Self::ResetStatsCost => P::CashboxSourceResetStatsCost,
            Self::DonationTax => P::CashboxSourceDonationTax,
            Self::CowardicePenalty => P::CashboxSourceCowardicePenalty,
            Self::BetCommission => P::CashboxSourceBetCommission,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StealProtection {
    pub item_key: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Copy)]
pub enum StealProtectionDuration {
    OneDay,
    ThreeDays,
    FiveDays,
    SevenDays,
}

impl StealProtectionDuration {
    fn as_proto(self) -> proto_coude::StealProtectionDurationKind {
        use proto_coude::StealProtectionDurationKind as P;
        match self {
            Self::OneDay => P::StealProtectionDurationOneDay,
            Self::ThreeDays => P::StealProtectionDurationThreeDays,
            Self::FiveDays => P::StealProtectionDurationFiveDays,
            Self::SevenDays => P::StealProtectionDurationSevenDays,
        }
    }

    pub fn days(self) -> i64 {
        match self {
            Self::OneDay => 1,
            Self::ThreeDays => 3,
            Self::FiveDays => 5,
            Self::SevenDays => 7,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::OneDay => "1 jour",
            Self::ThreeDays => "3 jours",
            Self::FiveDays => "5 jours",
            Self::SevenDays => "7 jours",
        }
    }

    pub fn from_key(s: &str) -> Option<Self> {
        match s {
            "1d" => Some(Self::OneDay),
            "3d" => Some(Self::ThreeDays),
            "5d" => Some(Self::FiveDays),
            "7d" => Some(Self::SevenDays),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StealProtectionTrigger {
    pub item_key: String,
    pub item_name: String,
    pub rolled_value: u32,
    pub block_chance_percent: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct CowardiceResponse {
    pub cowardice_count: i32,
}

// ══════════════════════════════════════════════════════════════════════
// ── API Client ──
// ══════════════════════════════════════════════════════════════════════

pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>, grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { base, grpc }
    }




    // Methodes braquage (Phase 10) deplacees dans `heist.rs`.

}

// ══════════════════════════════════════════════════════════════════════
// ── Helpers proto -> DTO ──
// ══════════════════════════════════════════════════════════════════════

fn proto_player_to_dto(p: proto_coude::CoudePlayer) -> Player {
    Player {
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
        class: p.class,
        title: p.title,
        // Fix bug /repos : sans ces champs le check cooldown cote bot
        // lisait toujours None et le joueur pouvait spam la commande.
        class_changed_at: p.class_changed_at,
        hp_current: Some(p.hp_current),
        hp_max: Some(p.hp_max),
        hp_last_regen: p.hp_last_regen,
        repos_last_used: p.repos_last_used,
        season: Some(p.season),
        created_at: p.created_at,
        updated_at: p.updated_at,
    }
}

fn proto_combat_to_dto(c: proto_coude::CoudeCombat) -> Combat {
    Combat {
        id: c.id,
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
        created_at: c.created_at,
        accepted_at: c.accepted_at,
        resolved_at: c.resolved_at,
    }
}

fn proto_prime_to_dto(p: proto_coude::CoudePrime) -> Prime {
    Prime {
        id: p.id,
        guild_id: p.guild_id,
        target_id: p.target_id,
        target_name: p.target_name,
        placed_by_id: p.placed_by_id,
        placed_by_name: p.placed_by_name,
        amount: p.amount,
        claimed: p.claimed,
        claimed_by_id: p.claimed_by_id,
        claimed_by_name: p.claimed_by_name,
        claimed_at: p.claimed_at,
        created_at: p.created_at,
    }
}

pub(in crate::modules::coude::api_client) use sentinel_shared::grpc_client::grpc_err_to_string;

// ── Sous-modules (refactor god-object api_client.rs) ──
mod heist;
mod steal_protections;
mod steal_boosts;
mod taunts;
mod cashbox;
mod bets;
mod primes_insurance;
mod inventory;
mod leaderboards;
mod combats;
mod players;
mod economy;
mod events;
mod utility;
