//! Methodes `ApiClient` des combats et des cooldowns associes.
//!
//! Inclut :
//! - CRUD combat (create, get, get_pending, resolve, set_betting,
//!   expire, set_defender_special, list_expired)
//! - `resolve_combat_now` : resolution instantanee (surprise /
//!   bloodbath / defense item) qui retourne un embed pret a poster
//! - `get_catalog` : recuperation du catalogue Coude (classes, shop,
//!   progression, matchmaking, anti-theft) au boot du bot
//! - Cooldowns genereriques (check / set) utilises par les commandes
//!   avec cooldown fonctionnel (ex. /voler).

use crate::shared::grpc_client::GrpcCallError;
use sentinel_proto::coude::v1 as proto_coude;

use super::{
    grpc_err_to_string, proto_combat_to_dto, taunt_event_from_proto, ApiClient, Combat,
    ResolvedCombatEmbed, ResolvedCombatEmbedField,
};

impl ApiClient {
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
        let c = crate::grpc_call!(self.grpc, coude_combats, create, req)?;
        Ok(proto_combat_to_dto(c))
    }

    pub async fn get_combat(&self, id: &str) -> Result<Option<Combat>, String> {
        let req = proto_coude::GetCombatRequest { id: id.to_string() };
        let result = crate::grpc_call!(@raw self.grpc, coude_combats, get, req);
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
        let r = crate::grpc_call!(self.grpc, coude_combats, get_pending_for_attacker, req)?;
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
        let r = crate::grpc_call!(self.grpc, coude_combats, get_pending_for_defender, req)?;
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
        crate::grpc_call!(@unit self.grpc, coude_combats, resolve, req)
    }

    /// Phase 8 : recupere le catalogue complet Coude (classes, shop,
    /// progression, matchmaking). Appele une fois au boot du bot, cache en
    /// memoire dans la TypeMap. Le bot ne contient plus aucune donnee
    /// metier en dur — tout vient de l'API.
    pub async fn get_catalog(
        &self,
    ) -> Result<crate::modules::coude::catalog::CatalogCache, String> {
        let req = proto_coude::Empty {};
        let resp = crate::grpc_call!(self.grpc, coude_social, get_catalog, req)?;
        Ok(crate::modules::coude::catalog::CatalogCache {
            classes: resp
                .classes
                .into_iter()
                .map(|c| crate::modules::coude::catalog::ClassInfo {
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
                .map(|i| crate::modules::coude::catalog::ShopItemInfo {
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
                .map(|l| crate::modules::coude::catalog::LevelEntry {
                    level: l.level,
                    title: l.title,
                    xp_cumul: l.xp_cumul,
                })
                .collect(),
            matchmaking_buckets: resp
                .matchmaking_buckets
                .into_iter()
                .map(|b| crate::modules::coude::catalog::MatchmakingBucket {
                    gap_min: b.gap_min,
                    gap_max: b.gap_max,
                    handicap: b.handicap,
                    blocked: b.blocked,
                })
                .collect(),
            anti_theft_items: resp
                .anti_theft_items
                .into_iter()
                .map(|a| crate::modules::coude::catalog::AntiTheftItem {
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
    pub async fn resolve_combat_now(&self, combat_id: &str) -> Result<ResolvedCombatEmbed, String> {
        let req = proto_coude::ResolveCombatNowRequest {
            combat_id: combat_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, coude_combats, resolve_combat_now, req)?;
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
            taunt_events: resp
                .taunt_events
                .into_iter()
                .map(taunt_event_from_proto)
                .collect(),
            vendetta_humiliation: resp
                .vendetta_humiliation
                .map(|h| super::VendettaHumiliation {
                    target_user_id: h.target_user_id,
                    challenger_user_id: h.challenger_user_id,
                }),
        })
    }

    pub async fn set_combat_betting(&self, id: &str, message_id: &str) -> Result<bool, String> {
        let req = proto_coude::SetBettingRequest {
            id: id.to_string(),
            message_id: message_id.to_string(),
        };
        let r = crate::grpc_call!(self.grpc, coude_combats, set_betting, req)?;
        Ok(r.transitioned)
    }

    pub async fn expire_combat(&self, id: &str) -> Result<(), String> {
        let req = proto_coude::ExpireCombatRequest { id: id.to_string() };
        crate::grpc_call!(@unit self.grpc, coude_combats, expire, req)
    }

    /// Annule un combat SEULEMENT s il est encore en `pending`.
    /// Contrairement a `expire_combat` qui ecrase n importe quel statut,
    /// cette methode evite de detruire un combat qui vient de passer en
    /// `betting` (accepte concurremment par le defenseur).
    pub async fn cancel_combat(&self, id: &str) -> Result<(), String> {
        let req = proto_coude::CancelCombatRequest { id: id.to_string() };
        crate::grpc_call!(@unit self.grpc, coude_combats, cancel, req)
    }

    pub async fn set_defender_special(&self, id: &str, item_key: &str) -> Result<(), String> {
        let req = proto_coude::SetDefenderSpecialRequest {
            id: id.to_string(),
            item_key: item_key.to_string(),
        };
        crate::grpc_call!(@unit self.grpc, coude_combats, set_defender_special, req)
    }

    pub async fn get_expired_combats(&self) -> Result<Vec<Combat>, String> {
        let req = proto_coude::Empty {};
        let list = crate::grpc_call!(self.grpc, coude_combats, list_expired_pending, req)?;
        Ok(list.combats.into_iter().map(proto_combat_to_dto).collect())
    }
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
        let r = crate::grpc_call!(self.grpc, coude_social, check_cooldown, req)?;
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
        crate::grpc_call!(@unit self.grpc, coude_social, set_cooldown, req)
    }
}
