//! Implementation gRPC du `TamagotchiService`. Wrappe `ManagePetsUseCase`.
//! Respect hexagonal : passe toujours par le port inbound, jamais le repo.

use std::sync::Arc;

use tokio_stream::wrappers::ReceiverStream;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use sentinel_proto::tamagotchi::v1 as proto;
use sentinel_proto::tamagotchi::v1::tamagotchi_service_server::TamagotchiService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::adapters::inbound::grpc::parse_uuid;
use sentinel_core::domain::entities::tamagotchi::pet::{xp_progress, Pet};
use crate::ports::inbound::tamagotchi::manage_pets::{
    CareCommand, CombatCommand, CreatePetCommand, ManagePetsUseCase, TrainCommand, VisitCommand,
};

pub struct TamagotchiGrpc {
    pub uc: Arc<dyn ManagePetsUseCase>,
}

#[tonic::async_trait]
impl TamagotchiService for TamagotchiGrpc {
    async fn create_pet(
        &self,
        request: Request<proto::CreatePetRequest>,
    ) -> Result<Response<proto::Pet>, Status> {
        let req = request.into_inner();
        let pet = self
            .uc
            .create(CreatePetCommand {
                guild_id: req.guild_id,
                owner_id: req.owner_id,
                name: req.name,
                species: req.species,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(pet_to_proto(pet)))
    }

    async fn get_pet(
        &self,
        request: Request<proto::GetPetRequest>,
    ) -> Result<Response<proto::GetPetResponse>, Status> {
        let req = request.into_inner();
        let pet = self
            .uc
            .get_by_owner(&req.guild_id, &req.owner_id)
            .await
            .map_err(domain_to_status)?;
        let pet = match pet {
            Some(p) => {
                // Charge le journal d'actions (utilise par l'historique cote bot).
                let events = self.uc.recent_events(p.id, 10).await.unwrap_or_default();
                let mut proto_pet = pet_to_proto(p);
                proto_pet.events = events
                    .into_iter()
                    .map(|e| proto::PetEvent {
                        kind: e.kind,
                        detail: e.detail,
                        created_at: e.created_at.to_rfc3339(),
                    })
                    .collect();
                Some(proto_pet)
            }
            None => None,
        };
        Ok(Response::new(proto::GetPetResponse { pet }))
    }

    async fn care_pet(
        &self,
        request: Request<proto::CareRequest>,
    ) -> Result<Response<proto::Pet>, Status> {
        let req = request.into_inner();
        let pet = self
            .uc
            .care(CareCommand {
                pet_id: parse_uuid(&req.pet_id)?,
                action: req.action,
                coin_cost: req.coin_cost,
                hunger_delta: req.hunger_delta,
                happiness_delta: req.happiness_delta,
                energy_delta: req.energy_delta,
                xp_gain: req.xp_gain,
                cooldown_secs: req.cooldown_secs,
                cure: req.cure,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(pet_to_proto(pet)))
    }

    async fn train_pet(
        &self,
        request: Request<proto::TrainRequest>,
    ) -> Result<Response<proto::Pet>, Status> {
        let req = request.into_inner();
        let pet = self
            .uc
            .train(TrainCommand {
                pet_id: parse_uuid(&req.pet_id)?,
                stat: req.stat,
                energy_cost: req.energy_cost,
                coin_cost: req.coin_cost,
                stat_gain: req.stat_gain,
                cooldown_secs: req.cooldown_secs,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(pet_to_proto(pet)))
    }

    async fn visit(
        &self,
        request: Request<proto::VisitRequest>,
    ) -> Result<Response<proto::VisitResult>, Status> {
        let req = request.into_inner();
        let res = self
            .uc
            .visit(VisitCommand {
                guild_id: req.guild_id,
                visitor_id: req.visitor_id,
                visitor_name: req.visitor_name,
                target_id: req.target_id,
                xp_reward: req.xp_reward,
                coins_reward: req.coins_reward,
                cooldown_secs: req.cooldown_secs,
                max_per_day: req.max_per_day,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::VisitResult {
            target_name: res.target_name,
            xp_reward: res.xp_reward,
            coins_reward: res.coins_reward,
        }))
    }

    async fn combat(
        &self,
        request: Request<proto::CombatRequest>,
    ) -> Result<Response<proto::CombatResult>, Status> {
        let req = request.into_inner();
        let res = self
            .uc
            .combat(CombatCommand {
                guild_id: req.guild_id,
                attacker_id: req.attacker_id,
                attacker_name: req.attacker_name,
                target_id: req.target_id,
                energy_cost: req.energy_cost,
                cooldown_secs: req.cooldown_secs,
                elo_k: req.elo_k,
                xp_win: req.xp_win,
                xp_loss: req.xp_loss,
                w_str: req.w_str,
                w_vit: req.w_vit,
                w_agi: req.w_agi,
                random_max: req.random_max,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CombatResult {
            attacker_won: res.attacker_won,
            attacker_power: res.attacker_power,
            defender_power: res.defender_power,
            defender_name: res.defender_name,
            attacker_new_elo: res.attacker_new_elo,
            attacker_elo_delta: res.attacker_elo_delta,
        }))
    }

    async fn set_card_location(
        &self,
        request: Request<proto::SetCardLocationRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .set_card_location(&req.guild_id, &req.owner_id, &req.channel_id, &req.message_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    type ListCardsStream = ListCardsStream;

    async fn list_cards(
        &self,
        request: Request<proto::ListCardsRequest>,
    ) -> Result<Response<Self::ListCardsStream>, Status> {
        let req = request.into_inner();
        // `limit` sert de taille de batch pour la lecture paginee interne.
        let batch = if req.limit > 0 { req.limit } else { 500 };
        let start_after = match req.after_id.as_deref() {
            Some(s) if !s.is_empty() => Some(parse_uuid(s)?),
            _ => None,
        };

        let uc = self.uc.clone();
        // Canal borne : applique un back-pressure naturel si le client consomme
        // lentement (l'envoi bloque tant que le buffer est plein).
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        tokio::spawn(async move {
            let mut after = start_after;
            loop {
                let pets = match uc.list_cards(batch, after).await {
                    Ok(p) => p,
                    Err(e) => {
                        let _ = tx.send(Err(domain_to_status(e))).await;
                        return;
                    }
                };
                if pets.is_empty() {
                    break;
                }
                let len = pets.len();
                // Curseur = id du dernier pet (Uuid Copy) avant consommation.
                after = pets.last().map(|p| p.id);
                for p in pets {
                    // Ignore les pets sans carte Discord enregistree.
                    let (Some(ch), Some(msg)) =
                        (p.card_channel_id.clone(), p.card_message_id.clone())
                    else {
                        continue;
                    };
                    let card = proto::Card {
                        card_channel_id: ch,
                        card_message_id: msg,
                        pet: Some(pet_to_proto(p)),
                    };
                    // Client deconnecte -> on arrete de paginer.
                    if tx.send(Ok(card)).await.is_err() {
                        return;
                    }
                }
                // Derniere page atteinte.
                if len < batch as usize {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

/// Type de stream renvoye par `ListCards` (server-streaming).
type ListCardsStream = ReceiverStream<Result<proto::Card, Status>>;

fn pet_to_proto(p: Pet) -> proto::Pet {
    let (xp_in_level, xp_for_level) = xp_progress(p.xp);
    proto::Pet {
        id: p.id.to_string(),
        guild_id: p.guild_id,
        owner_id: p.owner_id,
        name: p.name,
        species: p.species,
        level: p.level,
        xp: p.xp,
        xp_in_level,
        xp_for_level,
        born_at: p.born_at.to_rfc3339(),
        hunger: p.hunger,
        happiness: p.happiness,
        energy: p.energy,
        status: p.status.as_str().to_string(),
        str: p.str_,
        vit: p.vit,
        agi: p.agi,
        elo: p.elo,
        wins: p.wins,
        losses: p.losses,
        // Le tick/care n'a pas besoin du journal ici ; events charges
        // separement cote bot si besoin (historique). On laisse vide.
        events: Vec::new(),
    }
}
