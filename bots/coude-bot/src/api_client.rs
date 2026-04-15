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
use sentinel_shared::grpc_client::{GrpcCallError, SentinelGrpcClient};

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
}

#[derive(Debug, Clone)]
pub struct ResolvedCombatEmbedField {
    pub name: String,
    pub value: String,
    pub inline: bool,
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

    // ══════════════════════════════════════════════════════════════════
    // Players — gRPC (CoudePlayerService) + HTTP legacy pour les methodes
    // sans equivalent proto.
    // ══════════════════════════════════════════════════════════════════

    pub async fn get_or_create_player(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<Player, String> {
        let req = proto_coude::GetOrCreatePlayerRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
        };
        let mut client = self.grpc.coude_players();
        let p = self
            .grpc
            .guarded(|| async move {
                client.get_or_create_player(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(proto_player_to_dto(p))
    }

    pub async fn get_player(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Player>, String> {
        let req = proto_coude::GetPlayerRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_players();
        let result = self
            .grpc
            .guarded(|| async move { client.get_player(req).await.map(|r| r.into_inner()) })
            .await;
        match result {
            Ok(p) => Ok(Some(proto_player_to_dto(p))),
            Err(GrpcCallError::Status(s)) if s.code() == tonic::Code::NotFound => Ok(None),
            Err(e) => Err(grpc_err_to_string(e)),
        }
    }

    pub async fn update_player_class(
        &self,
        guild_id: &str,
        user_id: &str,
        class: &str,
    ) -> Result<(), String> {
        let req = proto_coude::UpdatePlayerClassRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            class: class.to_string(),
        };
        let mut client = self.grpc.coude_players();
        self.grpc
            .guarded(|| async move {
                client.update_player_class(req).await.map(|_| ())
            })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn add_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<(i64, i32, bool, i32), String> {
        let req = proto_coude::AddXpRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            amount,
        };
        let mut client = self.grpc.coude_players();
        let r = self
            .grpc
            .guarded(|| async move { client.add_xp(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok((r.new_xp, r.new_level, r.leveled_up, r.stat_points_gained))
    }

    // ── Players : HTTP legacy (pas d'equivalent proto) ──

    /// HTTP : pas dans les use cases exposes.
    pub async fn spend_stat_point(
        &self,
        guild_id: &str,
        user_id: &str,
        stat: &str,
    ) -> Result<Player, String> {
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/players/{user_id}/spend-stat"),
                &serde_json::json!({ "stat": stat }),
            )
            .await
    }

    /// HTTP : pas dans les use cases exposes.
    pub async fn reset_stats(
        &self,
        guild_id: &str,
        user_id: &str,
        cost: i64,
    ) -> Result<Player, String> {
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/players/{user_id}/reset-stats"),
                &serde_json::json!({ "cost": cost }),
            )
            .await
    }

    /// HTTP : pas dans les use cases exposes (fire-and-forget).
    pub async fn record_win(
        &self,
        guild_id: &str,
        user_id: &str,
        earned: i64,
        stolen: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/record-win"),
                &serde_json::json!({ "earned": earned, "stolen": stolen }),
            )
            .await;
        Ok(())
    }

    pub async fn record_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/record-loss"),
                &serde_json::json!({ "lost": lost }),
            )
            .await;
        Ok(())
    }

    pub async fn record_draw(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/record-draw"),
                &serde_json::json!({ "lost": lost }),
            )
            .await;
        Ok(())
    }

    pub async fn increment_cowardice(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i32, String> {
        let resp: CowardiceResponse = self
            .base
            .post_json(
                &format!("/api/coude/{guild_id}/players/{user_id}/increment-cowardice"),
                &serde_json::json!({}),
            )
            .await?;
        Ok(resp.cowardice_count)
    }

    pub async fn increment_chaos_events(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/increment-chaos"),
                &serde_json::json!({}),
            )
            .await;
        Ok(())
    }

    pub async fn record_coins_earned(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/coins-earned"),
                &serde_json::json!({ "amount": amount }),
            )
            .await;
        Ok(())
    }

    pub async fn record_coins_lost(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/coins-lost"),
                &serde_json::json!({ "amount": amount }),
            )
            .await;
        Ok(())
    }

    // ══════════════════════════════════════════════════════════════════
    // Social — gRPC (CoudeSocialService)
    // ══════════════════════════════════════════════════════════════════

    pub async fn get_current_season(&self, guild_id: &str) -> Result<CurrentSeason, String> {
        let req = proto_coude::CurrentSeasonRequest {
            guild_id: guild_id.to_string(),
        };
        let mut client = self.grpc.coude_social();
        let s = self
            .grpc
            .guarded(|| async move { client.current_season(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(CurrentSeason {
            season_number: s.season_number,
            started_at: s.started_at,
            ends_at: s.ends_at,
            days_remaining: s.days_remaining,
        })
    }

    // ══════════════════════════════════════════════════════════════════
    // Casino & Economy — gRPC (CoudeEconomyService)
    // ══════════════════════════════════════════════════════════════════

    pub async fn record_casino_win(
        &self,
        guild_id: &str,
        user_id: &str,
        gain: i64,
    ) -> Result<(), String> {
        let req = proto_coude::RecordCasinoWinRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            gain,
        };
        let mut client = self.grpc.coude_economy();
        self.grpc
            .guarded(|| async move { client.record_casino_win(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn record_casino_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), String> {
        let req = proto_coude::RecordCasinoLossRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            lost,
        };
        let mut client = self.grpc.coude_economy();
        self.grpc
            .guarded(|| async move { client.record_casino_loss(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn record_casino_faillite(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, String> {
        let req = proto_coude::RecordCasinoFailliteRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_economy();
        let r = self
            .grpc
            .guarded(|| async move {
                client.record_casino_faillite(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.cleared_coins)
    }

    pub async fn count_casino_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<u64, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_economy();
        let r = self
            .grpc
            .guarded(|| async move {
                client.count_casino_today(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value.max(0) as u64)
    }

    pub async fn sum_casino_gains_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_economy();
        let r = self
            .grpc
            .guarded(|| async move {
                client.sum_casino_gains_today(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value)
    }

    pub async fn count_steal_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<u64, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_economy();
        let r = self
            .grpc
            .guarded(|| async move {
                client.count_steal_today(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value.max(0) as u64)
    }

    pub async fn transfer_coins(
        &self,
        guild_id: &str,
        from_id: &str,
        to_id: &str,
        amount: i64,
    ) -> Result<(), String> {
        let req = proto_coude::TransferRequest {
            guild_id: guild_id.to_string(),
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            amount,
        };
        let mut client = self.grpc.coude_economy();
        self.grpc
            .guarded(|| async move { client.transfer(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn record_steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<i64, String> {
        let req = proto_coude::StealRequest {
            guild_id: guild_id.to_string(),
            thief_id: thief_id.to_string(),
            victim_id: victim_id.to_string(),
            amount,
        };
        let mut client = self.grpc.coude_economy();
        let r = self
            .grpc
            .guarded(|| async move { client.steal(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.stolen)
    }

    // ══════════════════════════════════════════════════════════════════
    // Combats — gRPC (CoudeCombatsService)
    // ══════════════════════════════════════════════════════════════════

    pub async fn create_combat(
        &self,
        guild_id: &str,
        channel_id: Option<&str>,
        attacker_id: &str,
        attacker_name: &str,
        defender_id: &str,
        defender_name: &str,
        mise: i64,
        special_attack: Option<&str>,
    ) -> Result<Combat, String> {
        let req = proto_coude::CreateCombatRequest {
            guild_id: guild_id.to_string(),
            channel_id: channel_id.map(str::to_string),
            attacker_id: attacker_id.to_string(),
            attacker_name: attacker_name.to_string(),
            defender_id: defender_id.to_string(),
            defender_name: defender_name.to_string(),
            mise,
            special_attack: special_attack.map(str::to_string),
        };
        let mut client = self.grpc.coude_combats();
        let c = self
            .grpc
            .guarded(|| async move { client.create(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(proto_combat_to_dto(c))
    }

    pub async fn get_combat(&self, id: &str) -> Result<Option<Combat>, String> {
        let req = proto_coude::GetCombatRequest {
            id: id.to_string(),
        };
        let mut client = self.grpc.coude_combats();
        let result = self
            .grpc
            .guarded(|| async move { client.get(req).await.map(|r| r.into_inner()) })
            .await;
        match result {
            Ok(c) => Ok(Some(proto_combat_to_dto(c))),
            Err(GrpcCallError::Status(s)) if s.code() == tonic::Code::NotFound => Ok(None),
            Err(e) => Err(grpc_err_to_string(e)),
        }
    }

    pub async fn get_pending_combat_for_attacker(
        &self,
        guild_id: &str,
        attacker_id: &str,
    ) -> Result<Option<Combat>, String> {
        let req = proto_coude::GetPendingForAttackerRequest {
            guild_id: guild_id.to_string(),
            attacker_id: attacker_id.to_string(),
        };
        let mut client = self.grpc.coude_combats();
        let r = self
            .grpc
            .guarded(|| async move {
                client.get_pending_for_attacker(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.combat.map(proto_combat_to_dto))
    }

    pub async fn get_pending_combat_for_defender(
        &self,
        guild_id: &str,
        defender_id: &str,
    ) -> Result<Option<Combat>, String> {
        let req = proto_coude::GetPendingForDefenderRequest {
            guild_id: guild_id.to_string(),
            defender_id: defender_id.to_string(),
        };
        let mut client = self.grpc.coude_combats();
        let r = self
            .grpc
            .guarded(|| async move {
                client.get_pending_for_defender(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.combat.map(proto_combat_to_dto))
    }

    pub async fn resolve_combat(
        &self,
        id: &str,
        status: &str,
        winner_id: Option<&str>,
        attacker_roll: Option<i32>,
        defender_roll: Option<i32>,
        chaos_event: Option<&str>,
        result_message: Option<&str>,
        coins_transferred: i64,
    ) -> Result<(), String> {
        let req = proto_coude::ResolveCombatRequest {
            id: id.to_string(),
            status: status.to_string(),
            winner_id: winner_id.map(str::to_string),
            attacker_roll,
            defender_roll,
            chaos_event: chaos_event.map(str::to_string),
            result_message: result_message.map(str::to_string),
            coins_transferred,
        };
        let mut client = self.grpc.coude_combats();
        self.grpc
            .guarded(|| async move { client.resolve(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    /// Phase 8 : recupere le catalogue complet Coude (classes, shop,
    /// progression, matchmaking). Appele une fois au boot du bot, cache en
    /// memoire dans la TypeMap. Le bot ne contient plus aucune donnee
    /// metier en dur — tout vient de l'API.
    pub async fn get_catalog(&self) -> Result<crate::catalog::CatalogCache, String> {
        let req = proto_coude::Empty {};
        let mut client = self.grpc.coude_social();
        let resp = self
            .grpc
            .guarded(|| async move {
                client.get_catalog(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(crate::catalog::CatalogCache {
            classes: resp
                .classes
                .into_iter()
                .map(|c| crate::catalog::ClassInfo {
                    name: c.name,
                    emoji: c.emoji,
                    base_atk: c.base_atk,
                    base_def: c.base_def,
                    atk_growth: c.atk_growth,
                    def_growth: c.def_growth,
                    dodge_chance: c.dodge_chance,
                    steal_bonus: c.steal_bonus,
                    description: c.description,
                    passif_key: c.passif_key,
                    passif_description: c.passif_description,
                    passif_reveal: c.passif_reveal,
                })
                .collect(),
            shop_items: resp
                .shop_items
                .into_iter()
                .map(|i| crate::catalog::ShopItemInfo {
                    key: i.key,
                    name: i.name,
                    emoji: i.emoji,
                    price: i.price,
                    description: i.description,
                    category: i.category,
                    heal_amount: i.heal_amount,
                })
                .collect(),
            level_table: resp
                .level_table
                .into_iter()
                .map(|l| crate::catalog::LevelEntry {
                    level: l.level,
                    title: l.title,
                    xp_cumul: l.xp_cumul,
                })
                .collect(),
            matchmaking_buckets: resp
                .matchmaking_buckets
                .into_iter()
                .map(|b| crate::catalog::MatchmakingBucket {
                    gap_min: b.gap_min,
                    gap_max: b.gap_max,
                    handicap: b.handicap,
                    blocked: b.blocked,
                })
                .collect(),
            anti_theft_items: resp
                .anti_theft_items
                .into_iter()
                .map(|a| crate::catalog::AntiTheftItem {
                    key: a.key,
                    block_chance_percent: a.block_chance_percent,
                })
                .collect(),
            max_level: resp.max_level,
            hp_base: resp.hp_base,
            hp_per_def: resp.hp_per_def,
        })
    }

    /// Phase 7 : resolution instantanee d'un combat (surprise / bloodbath /
    /// defense via item). L'API applique toute la logique metier et retourne
    /// un embed pret a poster.
    pub async fn resolve_combat_now(
        &self,
        combat_id: &str,
    ) -> Result<ResolvedCombatEmbed, String> {
        let req = proto_coude::ResolveCombatNowRequest {
            combat_id: combat_id.to_string(),
        };
        let mut client = self.grpc.coude_combats();
        let resp = self
            .grpc
            .guarded(|| async move {
                client.resolve_combat_now(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(ResolvedCombatEmbed {
            title: resp.title,
            description: resp.description,
            color: resp.color,
            fields: resp
                .fields
                .into_iter()
                .map(|f| ResolvedCombatEmbedField {
                    name: f.name,
                    value: f.value,
                    inline: f.inline,
                })
                .collect(),
        })
    }

    pub async fn set_combat_betting(&self, id: &str, message_id: &str) -> Result<bool, String> {
        let req = proto_coude::SetBettingRequest {
            id: id.to_string(),
            message_id: message_id.to_string(),
        };
        let mut client = self.grpc.coude_combats();
        let r = self
            .grpc
            .guarded(|| async move {
                client.set_betting(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.transitioned)
    }

    pub async fn expire_combat(&self, id: &str) -> Result<(), String> {
        let req = proto_coude::ExpireCombatRequest {
            id: id.to_string(),
        };
        let mut client = self.grpc.coude_combats();
        self.grpc
            .guarded(|| async move { client.expire(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn set_defender_special(
        &self,
        id: &str,
        item_key: &str,
    ) -> Result<(), String> {
        let req = proto_coude::SetDefenderSpecialRequest {
            id: id.to_string(),
            item_key: item_key.to_string(),
        };
        let mut client = self.grpc.coude_combats();
        self.grpc
            .guarded(|| async move { client.set_defender_special(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn get_expired_combats(&self) -> Result<Vec<Combat>, String> {
        let req = proto_coude::Empty {};
        let mut client = self.grpc.coude_combats();
        let list = self
            .grpc
            .guarded(|| async move {
                client.list_expired_pending(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list.combats.into_iter().map(proto_combat_to_dto).collect())
    }

    // ══════════════════════════════════════════════════════════════════
    // Bets — gRPC (CoudeBetsService)
    // ══════════════════════════════════════════════════════════════════

    pub async fn place_bet(
        &self,
        guild_id: &str,
        combat_id: &str,
        bettor_id: &str,
        bettor_name: &str,
        backed_id: &str,
        amount: i64,
    ) -> Result<(), String> {
        let req = proto_coude::PlaceBetRequest {
            guild_id: guild_id.to_string(),
            combat_id: combat_id.to_string(),
            bettor_id: bettor_id.to_string(),
            bettor_name: bettor_name.to_string(),
            backed_id: backed_id.to_string(),
            amount,
        };
        let mut client = self.grpc.coude_bets();
        self.grpc
            .guarded(|| async move { client.place(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn get_combat_bets(&self, combat_id: &str) -> Result<Vec<Bet>, String> {
        let req = proto_coude::ListForCombatRequest {
            combat_id: combat_id.to_string(),
        };
        let mut client = self.grpc.coude_bets();
        let list = self
            .grpc
            .guarded(|| async move {
                client.list_for_combat(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list
            .bets
            .into_iter()
            .map(|b| Bet {
                id: b.id.to_string(),
                combat_id: b.combat_id,
                bettor_id: b.bettor_id,
                bettor_name: b.bettor_name,
                backed_id: b.backed_id,
                amount: b.amount,
            })
            .collect())
    }

    pub async fn get_betting_combat_for_player(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Combat>, String> {
        let req = proto_coude::GetBettingForParticipantRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_combats();
        let r = self
            .grpc
            .guarded(|| async move {
                client.get_betting_for_participant(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.combat.map(proto_combat_to_dto))
    }

    pub async fn resolve_bets(
        &self,
        combat_id: &str,
        winner_id: Option<&str>,
    ) -> Result<(Vec<BetResult>, Option<FighterBetBonus>), String> {
        let req = proto_coude::ResolveBetsRequest {
            combat_id: combat_id.to_string(),
            winner_id: winner_id.map(str::to_string),
        };
        let mut client = self.grpc.coude_bets();
        let plan = self
            .grpc
            .guarded(|| async move { client.resolve(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        let results = plan
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
            .collect();
        let bonus = plan.fighter_bonus.map(|b| FighterBetBonus {
            winner_id: b.winner_id,
            winner_bonus: b.winner_bonus,
            loser_id: b.loser_id,
            loser_bonus: b.loser_bonus,
            total_pot: b.total_pot,
        });
        Ok((results, bonus))
    }

    pub async fn refund_bets(&self, combat_id: &str) -> Result<(usize, i64), String> {
        let req = proto_coude::RefundBetsRequest {
            combat_id: combat_id.to_string(),
        };
        let mut client = self.grpc.coude_bets();
        let s = self
            .grpc
            .guarded(|| async move { client.refund(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok((s.refunded_count as usize, s.refunded_total))
    }

    // ══════════════════════════════════════════════════════════════════
    // Cooldowns — gRPC (CoudeSocialService)
    // ══════════════════════════════════════════════════════════════════

    pub async fn check_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
    ) -> Result<Option<String>, String> {
        let req = proto_coude::CheckCooldownRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            action: action.to_string(),
        };
        let mut client = self.grpc.coude_social();
        let r = self
            .grpc
            .guarded(|| async move {
                client.check_cooldown(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.available_at)
    }

    pub async fn set_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
        duration_secs: i64,
    ) -> Result<(), String> {
        let req = proto_coude::SetCooldownRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            action: action.to_string(),
            duration_secs,
        };
        let mut client = self.grpc.coude_social();
        self.grpc
            .guarded(|| async move { client.set_cooldown(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    // ══════════════════════════════════════════════════════════════════
    // Primes & Insurance — gRPC (CoudeInventoryService)
    // ══════════════════════════════════════════════════════════════════

    pub async fn create_prime(
        &self,
        guild_id: &str,
        target_id: &str,
        target_name: &str,
        placed_by_id: &str,
        placed_by_name: &str,
        amount: i64,
    ) -> Result<Prime, String> {
        let req = proto_coude::CreatePrimeRequest {
            guild_id: guild_id.to_string(),
            target_id: target_id.to_string(),
            target_name: target_name.to_string(),
            placed_by_id: placed_by_id.to_string(),
            placed_by_name: placed_by_name.to_string(),
            amount,
        };
        let mut client = self.grpc.coude_inventory();
        let p = self
            .grpc
            .guarded(|| async move {
                client.create_prime(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(proto_prime_to_dto(p))
    }

    pub async fn get_active_primes(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Vec<Prime>, String> {
        let req = proto_coude::ListActivePrimesRequest {
            guild_id: guild_id.to_string(),
            target_id: target_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let list = self
            .grpc
            .guarded(|| async move {
                client.list_active_primes(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list.primes.into_iter().map(proto_prime_to_dto).collect())
    }

    pub async fn claim_primes(
        &self,
        guild_id: &str,
        target_id: &str,
        claimer_id: &str,
        claimer_name: &str,
    ) -> Result<i64, String> {
        let req = proto_coude::ClaimPrimesRequest {
            guild_id: guild_id.to_string(),
            target_id: target_id.to_string(),
            claimer_id: claimer_id.to_string(),
            claimer_name: claimer_name.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client.claim_primes(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value)
    }

    pub async fn buy_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
        duration_seconds: i64,
    ) -> Result<(), String> {
        let req = proto_coude::BuyInsuranceRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            is_scam,
            duration_seconds,
        };
        let mut client = self.grpc.coude_inventory();
        self.grpc
            .guarded(|| async move { client.buy_insurance(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn get_active_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Insurance>, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client.get_active_insurance(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.insurance.map(|i| Insurance {
            id: i.id,
            is_scam: i.is_scam,
            expires_at: i.expires_at,
        }))
    }

    pub async fn expire_insurance(&self, id: &str) -> Result<(), String> {
        let req = proto_coude::ExpireInsuranceRequest {
            insurance_id: id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        self.grpc
            .guarded(|| async move { client.expire_insurance(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    // ══════════════════════════════════════════════════════════════════
    // Phase 9 Part B : abonnements anti-vol
    // ══════════════════════════════════════════════════════════════════

    pub async fn list_active_steal_protections(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<StealProtection>, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client
                    .list_active_steal_protections(req)
                    .await
                    .map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.protections
            .into_iter()
            .map(|p| StealProtection {
                item_key: p.item_key,
                expires_at: p.expires_at,
            })
            .collect())
    }

    pub async fn price_steal_protection(
        &self,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<i64, String> {
        let req = proto_coude::PriceStealProtectionRequest {
            item_key: item_key.to_string(),
            duration: duration.as_proto() as i32,
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client.price_steal_protection(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value)
    }

    /// Achete un abonnement de protection. Retourne (cost, expires_at).
    pub async fn buy_steal_protection(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<(i64, String), String> {
        let req = proto_coude::BuyStealProtectionRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
            duration: duration.as_proto() as i32,
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client.buy_steal_protection(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok((r.cost, r.expires_at))
    }

    /// Interroge le serveur pour savoir si une protection a bloque un
    /// vol. L'API roll elle-meme les dés — le bot n'a aucun secret a
    /// garder (au contraire, le voleur ne voit meme pas le nom de
    /// l'item bloquant avant qu'il s'active).
    pub async fn try_trigger_steal_protection(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<StealProtectionTrigger>, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: target_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client
                    .try_trigger_steal_protection(req)
                    .await
                    .map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.trigger.map(|t| StealProtectionTrigger {
            item_key: t.item_key,
            item_name: t.item_name,
            rolled_value: t.rolled_value,
            block_chance_percent: t.block_chance_percent,
        }))
    }

    // ══════════════════════════════════════════════════════════════════
    // Phase 9 Part C : boost voleur
    // ══════════════════════════════════════════════════════════════════

    pub async fn price_steal_boost(
        &self,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<i64, String> {
        let req = proto_coude::PriceStealBoostRequest {
            item_key: item_key.to_string(),
            duration: duration.as_proto() as i32,
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move { client.price_steal_boost(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value)
    }

    pub async fn buy_steal_boost(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<(i64, String), String> {
        let req = proto_coude::BuyStealBoostRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
            duration: duration.as_proto() as i32,
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move { client.buy_steal_boost(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok((r.cost, r.expires_at))
    }

    /// Retourne la somme des roll bonuses des items de boost actifs du
    /// voleur. 0 si aucun item actif. Appele avant un /voler pour
    /// ajouter au thief_total.
    pub async fn get_steal_boost_total(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i32, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client.get_steal_boost_total(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value as i32)
    }

    // ══════════════════════════════════════════════════════════════════
    // Leaderboards — gRPC (CoudeSocialService)
    // ══════════════════════════════════════════════════════════════════

    async fn leaderboard(
        &self,
        guild_id: &str,
        category: proto_coude::LeaderboardCategory,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        let req = proto_coude::LeaderboardRequest {
            guild_id: guild_id.to_string(),
            category: category as i32,
            limit,
        };
        let mut client = self.grpc.coude_social();
        let list = self
            .grpc
            .guarded(|| async move { client.leaderboard(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list
            .entries
            .into_iter()
            .map(|e| LeaderboardEntry {
                user_id: e.user_id,
                username: e.username,
                value: e.value,
            })
            .collect())
    }

    pub async fn leaderboard_richest(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, proto_coude::LeaderboardCategory::Richest, limit)
            .await
    }

    pub async fn leaderboard_thieves(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, proto_coude::LeaderboardCategory::Thieves, limit)
            .await
    }

    pub async fn leaderboard_cowards(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, proto_coude::LeaderboardCategory::Cowards, limit)
            .await
    }

    pub async fn leaderboard_chaos(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, proto_coude::LeaderboardCategory::Chaos, limit)
            .await
    }

    pub async fn leaderboard_level(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, proto_coude::LeaderboardCategory::Level, limit)
            .await
    }

    // ══════════════════════════════════════════════════════════════════
    // HTTP legacy : methodes sans equivalent proto
    // ══════════════════════════════════════════════════════════════════

    pub async fn get_all_guild_ids(&self) -> Result<Vec<String>, String> {
        self.base.get_json("/api/coude/guilds").await
    }

    pub async fn get_wallet_transactions(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<WalletTransaction>, String> {
        self.base
            .get_json(&format!(
                "/api/wallet/{guild_id}/{user_id}/transactions?limit={limit}"
            ))
            .await
    }

    pub async fn get_random_players(
        &self,
        guild_id: &str,
        count: i64,
    ) -> Result<Vec<Player>, String> {
        self.base
            .get_json(&format!(
                "/api/coude/{guild_id}/players/random?count={count}"
            ))
            .await
    }

    // ══════════════════════════════════════════════════════════════════
    // Daily chaos + events — gRPC (CoudeSocialService)
    // ══════════════════════════════════════════════════════════════════

    pub async fn log_daily_chaos(
        &self,
        guild_id: &str,
        loser_id: &str,
        loser_name: &str,
        winner_id: &str,
        winner_name: &str,
        amount: i64,
    ) -> Result<(), String> {
        let req = proto_coude::LogDailyChaosRequest {
            guild_id: guild_id.to_string(),
            loser_id: loser_id.to_string(),
            loser_name: loser_name.to_string(),
            winner_id: winner_id.to_string(),
            winner_name: winner_name.to_string(),
            amount,
        };
        let mut client = self.grpc.coude_social();
        self.grpc
            .guarded(|| async move { client.log_daily_chaos(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn get_active_events(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ServerEvent>, String> {
        let req = proto_coude::ListActiveEventsRequest {
            guild_id: guild_id.to_string(),
        };
        let mut client = self.grpc.coude_social();
        let list = self
            .grpc
            .guarded(|| async move {
                client.list_active_events(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list
            .events
            .into_iter()
            .map(|e| ServerEvent {
                id: e.id,
                guild_id: e.guild_id,
                event_type: String::new(),
                active: e.active,
                expires_at: e.expires_at,
                created_at: e.created_at,
            })
            .collect())
    }

    // Phase 9 — Cagnotte communautaire.
    pub async fn get_cashbox(&self, guild_id: &str) -> Result<Cashbox, String> {
        let req = proto_coude::GetCashboxRequest {
            guild_id: guild_id.to_string(),
        };
        let mut client = self.grpc.coude_social();
        let r = self
            .grpc
            .guarded(|| async move { client.get_cashbox(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(Cashbox {
            guild_id: r.guild_id,
            balance: r.balance,
            total_collected: r.total_collected,
            total_redistributed: r.total_redistributed,
            last_redistribution_at: r.last_redistribution_at,
        })
    }

    /// Depose un montant dans la caisse communautaire. Best-effort :
    /// une erreur est journalisee mais ne bloque pas l'appelant, pour
    /// que l'achat principal n'echoue pas si la caisse est indisponible.
    pub async fn deposit_cashbox(
        &self,
        guild_id: &str,
        amount: i64,
        source: CashboxDepositSource,
    ) -> Result<(), String> {
        if amount <= 0 {
            return Ok(());
        }
        let req = proto_coude::DepositCashboxRequest {
            guild_id: guild_id.to_string(),
            amount,
            source: source.as_proto() as i32,
        };
        let mut client = self.grpc.coude_social();
        self.grpc
            .guarded(|| async move { client.deposit_cashbox(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    // ══════════════════════════════════════════════════════════════════
    // Inventory items — gRPC (CoudeInventoryService)
    // ══════════════════════════════════════════════════════════════════

    pub async fn add_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<(), String> {
        let req = proto_coude::AddItemRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        self.grpc
            .guarded(|| async move { client.add_item(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn get_inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<InventoryItem>, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let list = self
            .grpc
            .guarded(|| async move {
                client.list_inventory(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list
            .items
            .into_iter()
            .map(|i| InventoryItem {
                guild_id: i.guild_id,
                user_id: i.user_id,
                item_key: i.item_key,
                quantity: i.quantity,
            })
            .collect())
    }

    pub async fn use_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, String> {
        let req = proto_coude::UseItemRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move { client.use_item(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.consumed)
    }

    pub async fn has_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, String> {
        let req = proto_coude::HasItemRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move { client.has_item(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value)
    }

    // ══════════════════════════════════════════════════════════════════
    // Coins helpers (Phase 7A — CoudePlayerService.AdjustCoins)
    // ══════════════════════════════════════════════════════════════════

    pub async fn update_player_coins(
        &self,
        guild_id: &str,
        user_id: &str,
        delta: i64,
    ) -> Result<Player, String> {
        let req = proto_coude::AdjustCoinsRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            delta,
        };
        let mut client = self.grpc.coude_players();
        self.grpc
            .guarded(|| async move { client.adjust_coins(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)?;
        // Refresh player state.
        self.get_or_create_player(guild_id, user_id, "").await
    }

    pub async fn set_player_coins(
        &self,
        guild_id: &str,
        user_id: &str,
        coins: i64,
    ) -> Result<(), String> {
        let player = self.get_or_create_player(guild_id, user_id, "").await?;
        let delta = coins - player.coins;
        if delta != 0 {
            let req = proto_coude::AdjustCoinsRequest {
                guild_id: guild_id.to_string(),
                user_id: user_id.to_string(),
                delta,
            };
            let mut client = self.grpc.coude_players();
            self.grpc
                .guarded(|| async move { client.adjust_coins(req).await.map(|_| ()) })
                .await
                .map_err(grpc_err_to_string)?;
        }
        Ok(())
    }

    // ══════════════════════════════════════════════════════════════════
    // HP — gRPC (CoudePlayerService.UpdateHp)
    // ══════════════════════════════════════════════════════════════════

    pub async fn update_hp(
        &self,
        guild_id: &str,
        user_id: &str,
        hp_current: i32,
        hp_max: i32,
    ) -> Result<(), String> {
        let req = proto_coude::UpdateHpRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            hp_current,
            hp_max,
        };
        let mut client = self.grpc.coude_players();
        self.grpc
            .guarded(|| async move { client.update_hp(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    /// HTTP : pas d'equivalent proto.
    pub async fn repos(&self, guild_id: &str, user_id: &str) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/players/{user_id}/repos"),
                &serde_json::json!({}),
            )
            .await;
        Ok(())
    }
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
        class_changed_at: None,
        hp_current: Some(p.hp_current),
        hp_max: Some(p.hp_max),
        hp_last_regen: None,
        repos_last_used: None,
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

fn grpc_err_to_string(e: GrpcCallError) -> String {
    match e {
        GrpcCallError::Unavailable => "API indisponible (circuit breaker ouvert)".to_string(),
        GrpcCallError::Status(s) => format!("gRPC {:?}: {}", s.code(), s.message()),
        GrpcCallError::Transport(t) => format!("transport gRPC: {t}"),
    }
}
