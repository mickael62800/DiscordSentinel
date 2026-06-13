//! Handlers HTTP du jeu Tamagotchi.

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::entities::tamagotchi::pet::{xp_progress, Pet};
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::tamagotchi::manage_pets::{CareCommand, CreatePetCommand};

#[derive(Debug, Serialize)]
pub struct PetEventDto {
    pub kind: String,
    pub detail: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct PetDto {
    pub id: String,
    pub guild_id: String,
    pub owner_id: String,
    pub name: String,
    pub species: String,
    pub level: i32,
    pub xp: i64,
    pub xp_in_level: i64,
    pub xp_for_level: i64,
    pub born_at: String,
    pub hunger: i32,
    pub happiness: i32,
    pub energy: i32,
    pub status: String,
    pub str: i32,
    pub vit: i32,
    pub agi: i32,
    pub elo: i32,
    pub wins: i32,
    pub losses: i32,
    pub cooldowns: serde_json::Value,
    pub events: Vec<PetEventDto>,
}

impl PetDto {
    fn from(p: Pet, events: Vec<PetEventDto>) -> Self {
        let (xp_in_level, xp_for_level) = xp_progress(p.xp);
        PetDto {
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
            cooldowns: p.cooldowns,
            events,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreatePetBody {
    pub guild_id: String,
    pub owner_id: String,
    pub name: String,
    pub species: String,
}

/// POST /api/tamagotchi/pets
pub async fn create_pet(
    State(state): State<AppState>,
    Json(body): Json<CreatePetBody>,
) -> Result<Json<PetDto>, ApiError> {
    let pet = state
        .pets_uc
        .create(CreatePetCommand {
            guild_id: body.guild_id,
            owner_id: body.owner_id,
            name: body.name,
            species: body.species,
        })
        .await?;
    Ok(Json(PetDto::from(pet, vec![])))
}

/// GET /api/tamagotchi/{guild_id}/{owner_id}
pub async fn get_pet(
    State(state): State<AppState>,
    Path((guild_id, owner_id)): Path<(String, String)>,
) -> Result<Json<PetDto>, ApiError> {
    let pet = state
        .pets_uc
        .get_by_owner(&guild_id, &owner_id)
        .await?
        .ok_or_else(|| ApiError::from(DomainError::NotFound("aucun compagnon".into())))?;
    let events = load_events(&state, pet.id).await;
    Ok(Json(PetDto::from(pet, events)))
}

#[derive(Debug, Deserialize)]
pub struct CareBody {
    /// "feed" | "play" | "sleep" | "cuddle".
    pub action: String,
    #[serde(default)]
    pub coin_cost: i64,
    #[serde(default)]
    pub hunger_delta: i32,
    #[serde(default)]
    pub happiness_delta: i32,
    #[serde(default)]
    pub energy_delta: i32,
    #[serde(default)]
    pub xp_gain: i64,
    #[serde(default)]
    pub cooldown_secs: i64,
    #[serde(default)]
    pub cure: bool,
}

/// POST /api/tamagotchi/pets/{pet_id}/care
pub async fn care_pet(
    State(state): State<AppState>,
    Path(pet_id): Path<String>,
    Json(body): Json<CareBody>,
) -> Result<Json<PetDto>, ApiError> {
    let id = Uuid::parse_str(&pet_id)
        .map_err(|_| ApiError::from(DomainError::ValidationError("pet_id invalide".into())))?;
    let pet = state
        .pets_uc
        .care(CareCommand {
            pet_id: id,
            action: body.action,
            coin_cost: body.coin_cost,
            hunger_delta: body.hunger_delta,
            happiness_delta: body.happiness_delta,
            energy_delta: body.energy_delta,
            xp_gain: body.xp_gain,
            cooldown_secs: body.cooldown_secs,
            cure: body.cure,
        })
        .await?;
    let events = load_events(&state, pet.id).await;
    Ok(Json(PetDto::from(pet, events)))
}

#[derive(Debug, Deserialize)]
pub struct TrainBody {
    /// "str" | "vit" | "agi".
    pub stat: String,
    #[serde(default)]
    pub energy_cost: i32,
    #[serde(default)]
    pub coin_cost: i64,
    #[serde(default)]
    pub stat_gain: i32,
    #[serde(default)]
    pub cooldown_secs: i64,
}

/// POST /api/tamagotchi/pets/{pet_id}/train
pub async fn train_pet(
    State(state): State<AppState>,
    Path(pet_id): Path<String>,
    Json(body): Json<TrainBody>,
) -> Result<Json<PetDto>, ApiError> {
    let id = Uuid::parse_str(&pet_id)
        .map_err(|_| ApiError::from(DomainError::ValidationError("pet_id invalide".into())))?;
    let pet = state
        .pets_uc
        .train(crate::ports::inbound::tamagotchi::manage_pets::TrainCommand {
            pet_id: id,
            stat: body.stat,
            energy_cost: body.energy_cost,
            coin_cost: body.coin_cost,
            stat_gain: body.stat_gain,
            cooldown_secs: body.cooldown_secs,
        })
        .await?;
    let events = load_events(&state, pet.id).await;
    Ok(Json(PetDto::from(pet, events)))
}

#[derive(Debug, Serialize)]
pub struct TickSummary {
    pub processed: usize,
    pub sick: usize,
    pub died: usize,
    pub recovered: usize,
}

/// POST /api/tamagotchi/tick — appele par le worker. Applique la
/// decroissance + maladie/mort a tous les compagnons vivants, avec la config
/// de chaque guild.
pub async fn tick_all(State(state): State<AppState>) -> Result<Json<TickSummary>, ApiError> {
    use sentinel_core::domain::entities::tamagotchi::pet::{TickConfig, TickOutcome};
    use std::collections::HashMap;

    let pets = state.pets_uc.list_alive(500).await?;
    let mut cfg_cache: HashMap<String, TickConfig> = HashMap::new();
    let mut summary = TickSummary { processed: 0, sick: 0, died: 0, recovered: 0 };

    for pet in pets {
        let cfg = if let Some(c) = cfg_cache.get(&pet.guild_id) {
            *c
        } else {
            let c = load_tick_config(&state, &pet.guild_id).await;
            cfg_cache.insert(pet.guild_id.clone(), c);
            c
        };
        match state.pets_uc.tick(pet.id, cfg).await {
            Ok(TickOutcome::FellSick) => summary.sick += 1,
            Ok(TickOutcome::Died) => summary.died += 1,
            Ok(TickOutcome::Recovered) => summary.recovered += 1,
            _ => {}
        }
        summary.processed += 1;
    }
    Ok(Json(summary))
}

async fn load_tick_config(
    state: &AppState,
    guild_id: &str,
) -> sentinel_core::domain::entities::tamagotchi::pet::TickConfig {
    use sentinel_core::domain::entities::tamagotchi::pet::TickConfig;
    let entries = state
        .bot_config_repo
        .get_config(guild_id, "tamagotchi-bot")
        .await
        .unwrap_or_default();
    let num = |key: &str, default: i64| -> i64 {
        entries
            .iter()
            .find(|e| e.config_key == key)
            .and_then(|e| e.config_value.parse().ok())
            .unwrap_or(default)
    };
    TickConfig {
        hunger_decay_per_hour: num("hunger_decay_per_hour", 8) as i32,
        happiness_decay_per_hour: num("happiness_decay_per_hour", 5) as i32,
        energy_decay_per_hour: num("energy_decay_per_hour", 6) as i32,
        sick_after_secs: num("sick_after_hours", 12) * 3600,
        death_after_sick_secs: num("death_after_sick_hours", 24) * 3600,
        low_threshold: num("low_gauge_malus_threshold", 20) as i32,
    }
}

#[derive(Debug, Deserialize)]
pub struct VisitBody {
    pub guild_id: String,
    pub visitor_id: String,
    pub visitor_name: String,
    pub target_id: String,
    #[serde(default)]
    pub xp_reward: i64,
    #[serde(default)]
    pub coins_reward: i64,
    #[serde(default)]
    pub cooldown_secs: i64,
    #[serde(default)]
    pub max_per_day: i64,
}

#[derive(Debug, Serialize)]
pub struct VisitResultDto {
    pub target_name: String,
    pub xp_reward: i64,
    pub coins_reward: i64,
}

/// POST /api/tamagotchi/visit
pub async fn visit(
    State(state): State<AppState>,
    Json(body): Json<VisitBody>,
) -> Result<Json<VisitResultDto>, ApiError> {
    let r = state
        .pets_uc
        .visit(crate::ports::inbound::tamagotchi::manage_pets::VisitCommand {
            guild_id: body.guild_id,
            visitor_id: body.visitor_id,
            visitor_name: body.visitor_name,
            target_id: body.target_id,
            xp_reward: body.xp_reward,
            coins_reward: body.coins_reward,
            cooldown_secs: body.cooldown_secs,
            max_per_day: body.max_per_day,
        })
        .await?;
    Ok(Json(VisitResultDto {
        target_name: r.target_name,
        xp_reward: r.xp_reward,
        coins_reward: r.coins_reward,
    }))
}

async fn load_events(state: &AppState, pet_id: Uuid) -> Vec<PetEventDto> {
    state
        .pets_uc
        .recent_events(pet_id, 5)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|e| PetEventDto {
            kind: e.kind,
            detail: e.detail,
            created_at: e.created_at.to_rfc3339(),
        })
        .collect()
}
