//! Handlers HTTP du jeu Tamagotchi.
//!
//! Surface HTTP reduite : seuls l'admin web (liste + suppression) et le worker
//! (tick de cycle de vie) restent en HTTP. Toutes les interactions du bot
//! (creation, soins, entrainement, visite, combat, cartes) passent par le
//! `TamagotchiService` gRPC.

use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::Serialize;
use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::{require_role, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use axum::http::StatusCode;
use sentinel_core::domain::entities::tamagotchi::pet::{xp_progress, Pet};
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::errors::DomainError;

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

fn forbid(s: StatusCode, msg: &str) -> ApiError {
    ApiError(if s == StatusCode::FORBIDDEN {
        DomainError::Forbidden(msg.into())
    } else {
        DomainError::Internal(msg.into())
    })
}

/// GET /api/tamagotchi/{guild_id}/pets — liste tous les compagnons de la guild
/// (vue d'administration : dresseurs + evolution). Lecture : admin+.
pub async fn list_pets(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<PetDto>>, ApiError> {
    require_role(&ctx, Role::Admin).map_err(|s| forbid(s, "admin+ requis"))?;
    let pets = state.pets_uc.list_by_guild(&guild_id).await?;
    let dtos = pets.into_iter().map(|p| PetDto::from(p, vec![])).collect();
    Ok(Json(dtos))
}

/// DELETE /api/tamagotchi/{guild_id}/pets/{pet_id} — supprime un compagnon.
/// Action destructive : owner+.
pub async fn delete_pet(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    Path((guild_id, pet_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    require_role(&ctx, Role::Owner).map_err(|s| forbid(s, "owner+ requis"))?;
    let id = validation::parse_uuid("pet_id", &pet_id).map_err(ApiError)?;
    // Securite IDOR : le middleware valide l'appartenance au guild du path et
    // require_role l'Owner de CE guild — mais rien ne garantit que le pet_id
    // appartient a ce guild. Sans ce controle, un Owner de la guilde A pourrait
    // supprimer un pet de la guilde B via son UUID. On confirme donc que le pet
    // fait partie de la guilde du path avant de supprimer (404 sinon, pour ne
    // pas divulguer l'existence d'un pet d'une autre guilde).
    let belongs = state
        .pets_uc
        .list_by_guild(&guild_id)
        .await?
        .iter()
        .any(|p| p.id == id);
    if !belongs {
        return Err(ApiError(DomainError::NotFound(
            "compagnon introuvable dans cette guilde".into(),
        )));
    }
    state.pets_uc.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct TickSummary {
    pub processed: usize,
    pub sick: usize,
    pub died: usize,
    pub recovered: usize,
    /// Salons de compagnons morts fermes par la passe de reconciliation
    /// (morts dont l'evenement live avait ete rate). Distinct de `died` :
    /// n'implique aucune nouvelle transition ni DM.
    pub channels_reclaimed: usize,
}

/// POST /api/tamagotchi/tick — appele par le worker. Applique la
/// decroissance + maladie/mort a tous les compagnons vivants, avec la config
/// de chaque guild. Notifie le bot (DM + carte) via Redis sur transition.
pub async fn tick_all(State(state): State<AppState>) -> Result<Json<TickSummary>, ApiError> {
    use sentinel_core::domain::entities::tamagotchi::pet::{TickConfig, TickOutcome};
    use std::collections::HashMap;

    const BATCH: i64 = 500;

    let mut cfg_cache: HashMap<String, TickConfig> = HashMap::new();
    let mut summary = TickSummary {
        processed: 0,
        sick: 0,
        died: 0,
        recovered: 0,
        channels_reclaimed: 0,
    };

    // ── Passe de reconciliation (AVANT la boucle des vivants) ──────────────
    // Ferme les salons prives orphelins de compagnons deja morts lors d'un
    // tick PRECEDENT (worker/Redis indisponible a l'instant t, droits
    // manquants, pet cree avant la fonctionnalite...). On la joue EN PREMIER
    // pour ne voir que les morts anterieurs : un pet qui meurt CE tick est
    // encore vivant a ce stade (la boucle des vivants ci-dessous le fera
    // basculer et emettra l'evenement de mort normal, avec DM). On evite ainsi
    // tout double-emit / double-DM pour une mort fraiche.
    //
    // Chaque salon est referme via un evenement `tamagotchi_pet_status`
    // status="death" marque `reconcile: true` : le bot supprime le salon SANS
    // re-DM le proprietaire (il a deja recu le DM de mort a l'epoque).
    //
    // Idempotence (clear-on-emit) : on efface `card_channel_id` juste apres
    // l'emission, donc un pet n'est reconcilie qu'une fois. Tradeoff assume :
    // si la suppression echoue cote bot (droits manquants), le champ est deja
    // vide et on ne retentera pas. Une garantie clear-on-ack exigerait une
    // boucle d'accuse de reception ; on garde le chemin simple.
    let mut reconcile_after: Option<Uuid> = None;
    loop {
        let batch = state
            .pets_uc
            .list_dead_with_channel(BATCH, reconcile_after)
            .await?;
        if batch.is_empty() {
            break;
        }
        reconcile_after = batch.last().map(|p| p.id);
        let batch_len = batch.len();

        for pet in batch {
            state.broadcaster.broadcast(
                "tamagotchi_pet_status",
                serde_json::json!({
                    "guild_id": pet.guild_id,
                    "owner_id": pet.owner_id,
                    "pet_name": pet.name,
                    "species": pet.species,
                    "status": "death",
                    "card_channel_id": pet.card_channel_id,
                    "card_message_id": pet.card_message_id,
                    "reconcile": true,
                }),
            );
            state.pets_uc.clear_card_location(pet.id).await?;
            summary.channels_reclaimed += 1;
        }

        if batch_len < BATCH as usize {
            break;
        }
    }

    // Pagination par curseur `id` : couvre TOUS les compagnons vivants, sans
    // troncature silencieuse.
    let mut after_id: Option<Uuid> = None;

    loop {
        let batch = state.pets_uc.list_alive(BATCH, after_id).await?;
        if batch.is_empty() {
            break;
        }
        after_id = batch.last().map(|p| p.id);
        let batch_len = batch.len();

        for pet in batch {
            let cfg = if let Some(c) = cfg_cache.get(&pet.guild_id) {
                *c
            } else {
                let c = load_tick_config(&state, &pet.guild_id).await;
                cfg_cache.insert(pet.guild_id.clone(), c);
                c
            };
            if let Ok(
                outcome @ (TickOutcome::FellSick | TickOutcome::Died | TickOutcome::Recovered),
            ) = state.pets_uc.tick(pet.id, cfg).await
            {
                match outcome {
                    TickOutcome::FellSick => summary.sick += 1,
                    TickOutcome::Died => summary.died += 1,
                    TickOutcome::Recovered => summary.recovered += 1,
                    _ => unreachable!(),
                }
                let status = match outcome {
                    TickOutcome::FellSick => "sick",
                    TickOutcome::Died => "death",
                    _ => "recovered",
                };
                state.broadcaster.broadcast(
                    "tamagotchi_pet_status",
                    serde_json::json!({
                        "guild_id": pet.guild_id,
                        "owner_id": pet.owner_id,
                        "pet_name": pet.name,
                        "species": pet.species,
                        "status": status,
                        "card_channel_id": pet.card_channel_id,
                        "card_message_id": pet.card_message_id,
                    }),
                );
            }
            summary.processed += 1;
        }

        if batch_len < BATCH as usize {
            break;
        }
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
