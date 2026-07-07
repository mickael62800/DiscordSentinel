//! Methodes `ApiClient` du domaine Players.
//!
//! Couvre toutes les operations sur les joueurs Coup de Coude :
//! - CRUD de base (get_or_create, get)
//! - Gameplay stats (class, xp, stat_points, reset)
//! - Record d'evenements (win/loss/draw, coins earned/lost,
//!   cowardice/chaos counters)
//! - Coins helpers (update_player_coins, set_player_coins)
//! - HP (update_hp, repos)
//!
//! La majorite des methodes passent par gRPC (`CoudePlayerService`).
//! Quelques methodes legacy restent en HTTP (POST fire-and-forget)
//! parce qu'elles n'ont pas d'equivalent proto : record_win/loss/draw,
//! increment_cowardice/chaos, record_coins_earned/lost, spend_stat_point,
//! reset_stats, repos.

use crate::shared::grpc_client::GrpcCallError;
use sentinel_proto::coude::v1 as proto_coude;

use super::{grpc_err_to_string, proto_player_to_dto, ApiClient, CowardiceResponse, Player};

/// Un palier de niveau (donnees d'affichage + statut), resolu server-side.
#[derive(Debug, Clone)]
pub struct MilestoneInfo {
    pub level: i32,
    pub key: String,
    pub label: String,
    pub emoji: String,
    pub description: String,
    pub unlocked: bool,
}

/// Etat de progression derive server-side (succes + paliers + cooldown).
#[derive(Debug, Clone)]
pub struct PlayerProgression {
    pub unlocked_achievements: Vec<String>,
    pub total_achievements: i32,
    pub milestones: Vec<MilestoneInfo>,
    pub next_milestone: Option<MilestoneInfo>,
    pub effective_repos_cooldown_hours: i64,
}

impl ApiClient {
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
        let p = crate::grpc_call!(self.grpc, coude_players, get_or_create_player, req)?;
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
        let result = crate::grpc_call!(@raw self.grpc, coude_players, get_player, req);
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
        crate::grpc_call!(@unit self.grpc, coude_players, update_player_class, req)
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
        let r = crate::grpc_call!(self.grpc, coude_players, add_xp, req)?;
        Ok((r.new_xp, r.new_level, r.leveled_up, r.stat_points_gained))
    }

    /// Progression derivee server-side (succes debloques + paliers + cooldown
    /// /repos effectif). Le bareme vit dans l'API ; le bot ne fait que rendre.
    pub async fn get_progression(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<PlayerProgression, String> {
        let req = proto_coude::GetPlayerRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let r = crate::grpc_call!(self.grpc, coude_players, get_progression, req)?;
        let map = |m: proto_coude::MilestoneInfo| MilestoneInfo {
            level: m.level,
            key: m.key,
            label: m.label,
            emoji: m.emoji,
            description: m.description,
            unlocked: m.unlocked,
        };
        Ok(PlayerProgression {
            unlocked_achievements: r.unlocked_achievements,
            total_achievements: r.total_achievements,
            milestones: r.milestones.into_iter().map(map).collect(),
            next_milestone: r.next_milestone.map(map),
            effective_repos_cooldown_hours: r.effective_repos_cooldown_hours,
        })
    }

    /// Cooldown /repos effectif (heures) pour ce joueur (regle palier niveau
    /// 15 appliquee server-side).
    pub async fn effective_repos_cooldown_hours(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, String> {
        let req = proto_coude::GetPlayerRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let r = crate::grpc_call!(
            self.grpc,
            coude_players,
            get_effective_repos_cooldown_hours,
            req
        )?;
        Ok(r.value)
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

    /// HTTP : pas dans les use cases exposes. Le cout du reset
    /// (`reset_stats_cost`) est lu server-side depuis la config guild.
    pub async fn reset_stats(&self, guild_id: &str, user_id: &str) -> Result<Player, String> {
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/players/{user_id}/reset-stats"),
                &serde_json::json!({}),
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

    pub async fn increment_cowardice(&self, guild_id: &str, user_id: &str) -> Result<i32, String> {
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
        crate::grpc_call!(@unit self.grpc, coude_players, adjust_coins, req)?;
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
            crate::grpc_call!(@unit self.grpc, coude_players, adjust_coins, req)?;
        }
        Ok(())
    }

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
        crate::grpc_call!(@unit self.grpc, coude_players, update_hp, req)
    }

    /// HTTP : pas d'equivalent proto. On attend la reponse complete
    /// (pas de fire-and-forget) — sinon un `/coude` lance juste apres
    /// lit des HP stale avant que le heal ne soit commit en BDD.
    pub async fn repos(&self, guild_id: &str, user_id: &str) -> Result<(), String> {
        let path = format!("/api/coude/{guild_id}/players/{user_id}/repos");
        let req = self
            .base
            .client()
            .post(format!("{}{}", self.base.base_url(), path))
            .json(&serde_json::json!({}));
        let resp = self
            .base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau POST {path}: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Erreur API {status} POST {path}: {text}"));
        }
        Ok(())
    }
}
