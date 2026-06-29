//! Client gRPC du module tamagotchi (`TamagotchiService`).
//!
//! Remplace les appels HTTP (`BaseApiClient`) pour les compagnons. La config
//! guild et le wallet restent sur HTTP (`BaseApiClient`), domaines distincts.

use std::sync::Arc;

use serenity::prelude::TypeMap;
use tracing::warn;

use crate::shared::grpc_client::{grpc_err_to_string, GrpcClientKey, SentinelGrpcClient};

use sentinel_proto::tamagotchi::v1 as proto;

/// Donnees d'un compagnon cote bot (mappees depuis `proto::Pet`).
#[derive(Debug, Clone, Default)]
pub struct PetData {
    pub id: String,
    pub name: String,
    pub species: String,
    pub born_at: String,
    pub level: i32,
    pub xp_in_level: i64,
    pub xp_for_level: i64,
    pub hunger: i32,
    pub happiness: i32,
    pub energy: i32,
    pub status: String,
    pub str_: i32,
    pub vit: i32,
    pub agi: i32,
    pub elo: i32,
    pub wins: i32,
    pub losses: i32,
    pub events: Vec<String>,
}

impl From<proto::Pet> for PetData {
    fn from(p: proto::Pet) -> Self {
        Self {
            id: p.id,
            name: p.name,
            species: p.species,
            born_at: p.born_at,
            level: p.level,
            xp_in_level: p.xp_in_level,
            xp_for_level: p.xp_for_level,
            hunger: p.hunger,
            happiness: p.happiness,
            energy: p.energy,
            status: p.status,
            str_: p.str,
            vit: p.vit,
            agi: p.agi,
            elo: p.elo,
            wins: p.wins,
            losses: p.losses,
            events: p.events.into_iter().map(|e| e.detail).collect(),
        }
    }
}

pub struct VisitData {
    pub target_name: String,
    pub xp_reward: i64,
    pub coins_reward: i64,
}

pub struct CombatData {
    pub attacker_won: bool,
    pub attacker_power: i64,
    pub defender_power: i64,
    pub defender_name: String,
    pub attacker_new_elo: i32,
    pub attacker_elo_delta: i32,
}

/// Une carte a rafraichir (refresh horaire).
pub struct CardData {
    pub guild_id: String,
    pub owner_id: String,
    pub card_channel_id: String,
    pub card_message_id: String,
    pub pet: PetData,
}

/// Arguments d'une action de soin (effets/couts calcules cote bot depuis la
/// config guild — appliques atomiquement cote API).
pub struct CareArgs {
    pub action: String,
    pub coin_cost: i64,
    pub hunger_delta: i32,
    pub happiness_delta: i32,
    pub energy_delta: i32,
    pub xp_gain: i64,
    pub cooldown_secs: i64,
    pub cure: bool,
}

pub struct TrainArgs {
    pub stat: String,
    pub energy_cost: i32,
    pub coin_cost: i64,
    pub stat_gain: i32,
    pub cooldown_secs: i64,
}

pub struct VisitArgs {
    pub guild_id: String,
    pub visitor_id: String,
    pub visitor_name: String,
    pub target_id: String,
    pub xp_reward: i64,
    pub coins_reward: i64,
    pub cooldown_secs: i64,
    pub max_per_day: i64,
}

pub struct CombatArgs {
    pub guild_id: String,
    pub attacker_id: String,
    pub attacker_name: String,
    pub target_id: String,
    pub energy_cost: i32,
    pub cooldown_secs: i64,
    pub elo_k: i32,
    pub xp_win: i64,
    pub xp_loss: i64,
    pub w_str: i32,
    pub w_vit: i32,
    pub w_agi: i32,
    pub random_max: i32,
}

/// Client gRPC tamagotchi. Cloneable (Channel = Arc en interne).
pub struct TamaApi {
    grpc: Arc<SentinelGrpcClient>,
}

impl TamaApi {
    pub fn from_data(data: &TypeMap) -> Option<Self> {
        let grpc = data.get::<GrpcClientKey>()?.clone();
        Some(Self { grpc })
    }

    pub async fn create_pet(
        &self,
        guild_id: &str,
        owner_id: &str,
        name: &str,
        species: &str,
    ) -> Result<PetData, String> {
        let req = proto::CreatePetRequest {
            guild_id: guild_id.to_string(),
            owner_id: owner_id.to_string(),
            name: name.to_string(),
            species: species.to_string(),
        };
        let g = &self.grpc;
        let mut c = g.tamagotchi();
        g.guarded(|| async move { c.create_pet(req).await.map(|r| r.into_inner()) })
            .await
            .map(PetData::from)
            .map_err(grpc_err_to_string)
    }

    /// `None` si le joueur n'a pas de compagnon.
    pub async fn get_pet(&self, guild_id: &str, owner_id: &str) -> Option<PetData> {
        let req = proto::GetPetRequest {
            guild_id: guild_id.to_string(),
            owner_id: owner_id.to_string(),
        };
        let g = &self.grpc;
        let mut c = g.tamagotchi();
        let resp = g
            .guarded(|| async move { c.get_pet(req).await.map(|r| r.into_inner()) })
            .await
            .ok()?;
        resp.pet.map(PetData::from)
    }

    pub async fn care(&self, pet_id: &str, args: CareArgs) -> Result<PetData, String> {
        let req = proto::CareRequest {
            pet_id: pet_id.to_string(),
            action: args.action,
            coin_cost: args.coin_cost,
            hunger_delta: args.hunger_delta,
            happiness_delta: args.happiness_delta,
            energy_delta: args.energy_delta,
            xp_gain: args.xp_gain,
            cooldown_secs: args.cooldown_secs,
            cure: args.cure,
        };
        let g = &self.grpc;
        let mut c = g.tamagotchi();
        g.guarded(|| async move { c.care_pet(req).await.map(|r| r.into_inner()) })
            .await
            .map(PetData::from)
            .map_err(grpc_err_to_string)
    }

    pub async fn train(&self, pet_id: &str, args: TrainArgs) -> Result<PetData, String> {
        let req = proto::TrainRequest {
            pet_id: pet_id.to_string(),
            stat: args.stat,
            energy_cost: args.energy_cost,
            coin_cost: args.coin_cost,
            stat_gain: args.stat_gain,
            cooldown_secs: args.cooldown_secs,
        };
        let g = &self.grpc;
        let mut c = g.tamagotchi();
        g.guarded(|| async move { c.train_pet(req).await.map(|r| r.into_inner()) })
            .await
            .map(PetData::from)
            .map_err(grpc_err_to_string)
    }

    pub async fn visit(&self, args: VisitArgs) -> Result<VisitData, String> {
        let req = proto::VisitRequest {
            guild_id: args.guild_id,
            visitor_id: args.visitor_id,
            visitor_name: args.visitor_name,
            target_id: args.target_id,
            xp_reward: args.xp_reward,
            coins_reward: args.coins_reward,
            cooldown_secs: args.cooldown_secs,
            max_per_day: args.max_per_day,
        };
        let g = &self.grpc;
        let mut c = g.tamagotchi();
        g.guarded(|| async move { c.visit(req).await.map(|r| r.into_inner()) })
            .await
            .map(|r| VisitData {
                target_name: r.target_name,
                xp_reward: r.xp_reward,
                coins_reward: r.coins_reward,
            })
            .map_err(grpc_err_to_string)
    }

    pub async fn combat(&self, args: CombatArgs) -> Result<CombatData, String> {
        let req = proto::CombatRequest {
            guild_id: args.guild_id,
            attacker_id: args.attacker_id,
            attacker_name: args.attacker_name,
            target_id: args.target_id,
            energy_cost: args.energy_cost,
            cooldown_secs: args.cooldown_secs,
            elo_k: args.elo_k,
            xp_win: args.xp_win,
            xp_loss: args.xp_loss,
            w_str: args.w_str,
            w_vit: args.w_vit,
            w_agi: args.w_agi,
            random_max: args.random_max,
        };
        let g = &self.grpc;
        let mut c = g.tamagotchi();
        g.guarded(|| async move { c.combat(req).await.map(|r| r.into_inner()) })
            .await
            .map(|r| CombatData {
                attacker_won: r.attacker_won,
                attacker_power: r.attacker_power,
                defender_power: r.defender_power,
                defender_name: r.defender_name,
                attacker_new_elo: r.attacker_new_elo,
                attacker_elo_delta: r.attacker_elo_delta,
            })
            .map_err(grpc_err_to_string)
    }

    /// Enregistre la position de la carte Discord (fire-and-forget tolerant).
    pub async fn set_card_location(
        &self,
        guild_id: &str,
        owner_id: &str,
        channel_id: u64,
        message_id: u64,
    ) {
        let req = proto::SetCardLocationRequest {
            guild_id: guild_id.to_string(),
            owner_id: owner_id.to_string(),
            channel_id: channel_id.to_string(),
            message_id: message_id.to_string(),
        };
        let g = &self.grpc;
        let mut c = g.tamagotchi();
        if let Err(e) = g
            .guarded(|| async move { c.set_card_location(req).await.map(|_| ()) })
            .await
        {
            warn!(error = %grpc_err_to_string(e), "Echec set_card_location gRPC");
        }
    }

    /// Toutes les cartes vivantes a rafraichir, consommees depuis le
    /// server-stream `ListCards` (le serveur pagine la lecture DB en interne).
    /// Le circuit breaker garde l'ouverture de l'appel ; les erreurs survenant
    /// pendant la consommation du stream sont remontees telles quelles.
    pub async fn list_cards(&self) -> Result<Vec<CardData>, String> {
        // limit = taille de batch cote serveur ; le client itere le stream.
        let req = proto::ListCardsRequest {
            limit: 500,
            after_id: None,
        };
        let g = &self.grpc;
        let mut c = g.tamagotchi();
        let mut stream = g
            .guarded(|| async move { c.list_cards(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;

        let mut out = Vec::new();
        while let Some(card) = stream
            .message()
            .await
            .map_err(|e| format!("stream tamagotchi: {e}"))?
        {
            let Some(pet) = card.pet else { continue };
            out.push(CardData {
                guild_id: pet.guild_id.clone(),
                owner_id: pet.owner_id.clone(),
                card_channel_id: card.card_channel_id,
                card_message_id: card.card_message_id,
                pet: PetData::from(pet),
            });
        }
        Ok(out)
    }
}
