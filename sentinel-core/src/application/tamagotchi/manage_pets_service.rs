//! Service application Tamagotchi : creation + actions de soin.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::tamagotchi::pet::{
    Health, NewPet, Pet, PetEvent, TickConfig, TickOutcome,
};
use crate::domain::entities::tamagotchi::species::Species;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::tamagotchi::manage_pets::{
    CareCommand, CreatePetCommand, ManagePetsUseCase,
};
use crate::ports::outbound::tamagotchi::pet_repository::PetRepository;

pub struct ManagePetsService {
    repo: Arc<dyn PetRepository>,
    wallet: Arc<dyn ManageWalletUseCase>,
}

impl ManagePetsService {
    pub fn new(repo: Arc<dyn PetRepository>, wallet: Arc<dyn ManageWalletUseCase>) -> Self {
        Self { repo, wallet }
    }
}

fn clamp_gauge(v: i32) -> i32 {
    v.clamp(0, 100)
}

#[async_trait]
impl ManagePetsUseCase for ManagePetsService {
    async fn create(&self, cmd: CreatePetCommand) -> Result<Pet, DomainError> {
        if cmd.guild_id.trim().is_empty() || cmd.owner_id.trim().is_empty() {
            return Err(DomainError::ValidationError("guild_id et owner_id requis".into()));
        }
        let name = cmd.name.trim();
        if name.is_empty() || name.chars().count() > 32 {
            return Err(DomainError::ValidationError(
                "nom requis (1-32 caracteres)".into(),
            ));
        }
        let species = Species::from_str(&cmd.species)
            .ok_or_else(|| DomainError::ValidationError(format!("espece inconnue : {}", cmd.species)))?;
        // Un seul compagnon vivant par joueur.
        if let Some(existing) = self.repo.get_by_owner(&cmd.guild_id, &cmd.owner_id).await? {
            if existing.status != Health::Dead {
                return Err(DomainError::Conflict("tu as deja un compagnon".into()));
            }
        }
        let base = species.base_stats();
        let pet = self
            .repo
            .create(NewPet {
                guild_id: cmd.guild_id,
                owner_id: cmd.owner_id,
                name: name.to_string(),
                species: species.as_str().to_string(),
                str_: base.str_,
                vit: base.vit,
                agi: base.agi,
            })
            .await?;
        let _ = self
            .repo
            .add_event(pet.id, "born", &format!("{} ({}) est ne !", pet.name, species.display()))
            .await;
        Ok(pet)
    }

    async fn get_by_owner(&self, guild_id: &str, owner_id: &str) -> Result<Option<Pet>, DomainError> {
        self.repo.get_by_owner(guild_id, owner_id).await
    }

    async fn recent_events(&self, pet_id: Uuid, limit: i64) -> Result<Vec<PetEvent>, DomainError> {
        self.repo.recent_events(pet_id, limit.clamp(1, 50)).await
    }

    async fn care(&self, cmd: CareCommand) -> Result<Pet, DomainError> {
        let mut pet = self
            .repo
            .get(cmd.pet_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("compagnon introuvable".into()))?;

        if pet.status == Health::Dead {
            return Err(DomainError::Conflict("ton compagnon est mort".into()));
        }

        let now = Utc::now();
        let remaining = pet.cooldown_remaining_secs(&cmd.action, now, cmd.cooldown_secs);
        if remaining > 0 {
            return Err(DomainError::Conflict(format!(
                "action en cooldown ({remaining}s restantes)"
            )));
        }

        // Debit coins (wallet partage) si l'action a un cout.
        if cmd.coin_cost > 0 {
            self.wallet
                .debit(
                    &pet.guild_id,
                    &pet.owner_id,
                    cmd.coin_cost,
                    "tamagotchi",
                    &format!("Tamagotchi : {}", cmd.action),
                )
                .await?;
        }

        // Applique les effets.
        pet.hunger = clamp_gauge(pet.hunger + cmd.hunger_delta);
        pet.happiness = clamp_gauge(pet.happiness + cmd.happiness_delta);
        pet.energy = clamp_gauge(pet.energy + cmd.energy_delta);
        if cmd.xp_gain > 0 {
            pet.xp += cmd.xp_gain;
            pet.refresh_level();
        }
        pet.set_cooldown(&cmd.action, now);

        let saved = self.repo.save(&pet).await?;
        let _ = self
            .repo
            .add_event(saved.id, &cmd.action, &format!("Action : {}", cmd.action))
            .await;
        Ok(saved)
    }

    async fn list_alive(&self, limit: i64) -> Result<Vec<Pet>, DomainError> {
        self.repo.list_alive(limit.clamp(1, 500)).await
    }

    async fn tick(&self, pet_id: Uuid, cfg: TickConfig) -> Result<TickOutcome, DomainError> {
        let mut pet = match self.repo.get(pet_id).await? {
            Some(p) => p,
            None => return Ok(TickOutcome::Unchanged),
        };
        let outcome = pet.apply_tick(Utc::now(), &cfg);
        if outcome == TickOutcome::Unchanged {
            return Ok(outcome);
        }
        self.repo.save(&pet).await?;
        let detail = match outcome {
            TickOutcome::FellSick => Some(format!("{} est tombe malade !", pet.name)),
            TickOutcome::Died => Some(format!("{} est mort... 🪦", pet.name)),
            TickOutcome::Recovered => Some(format!("{} est gueri.", pet.name)),
            _ => None,
        };
        if let Some(d) = detail {
            let kind = match outcome {
                TickOutcome::FellSick => "sick",
                TickOutcome::Died => "death",
                TickOutcome::Recovered => "recovered",
                _ => "tick",
            };
            let _ = self.repo.add_event(pet.id, kind, &d).await;
        }
        Ok(outcome)
    }
}
