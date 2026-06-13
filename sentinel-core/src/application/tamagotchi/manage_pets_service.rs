//! Service application Tamagotchi : creation + actions de soin.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::tamagotchi::pet::{
    combat_power, elo_update, Health, NewPet, Pet, PetEvent, TickConfig, TickOutcome,
};
use crate::domain::entities::tamagotchi::species::Species;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::tamagotchi::manage_pets::{
    CareCommand, CombatCommand, CombatResult, CreatePetCommand, ManagePetsUseCase, TrainCommand,
    VisitCommand, VisitResult,
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
        // Potion de soin : guerit la maladie.
        if cmd.cure && pet.status == Health::Sick {
            pet.status = Health::Healthy;
            pet.sick_since = None;
        }
        if pet.hunger > 0 {
            pet.hunger_zero_since = None;
        }
        if cmd.cooldown_secs > 0 {
            pet.set_cooldown(&cmd.action, now);
        }

        let saved = self.repo.save(&pet).await?;
        let _ = self
            .repo
            .add_event(saved.id, &cmd.action, &format!("Action : {}", cmd.action))
            .await;
        Ok(saved)
    }

    async fn train(&self, cmd: TrainCommand) -> Result<Pet, DomainError> {
        let mut pet = self
            .repo
            .get(cmd.pet_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("compagnon introuvable".into()))?;
        if pet.status == Health::Dead {
            return Err(DomainError::Conflict("ton compagnon est mort".into()));
        }
        let now = Utc::now();
        let remaining = pet.cooldown_remaining_secs("train", now, cmd.cooldown_secs);
        if remaining > 0 {
            return Err(DomainError::Conflict(format!(
                "entrainement en cooldown ({remaining}s restantes)"
            )));
        }
        if pet.energy < cmd.energy_cost {
            return Err(DomainError::Conflict("ton compagnon est epuise".into()));
        }
        if cmd.coin_cost > 0 {
            self.wallet
                .debit(&pet.guild_id, &pet.owner_id, cmd.coin_cost, "tamagotchi", "Tamagotchi : entrainement")
                .await?;
        }
        pet.energy = clamp_gauge(pet.energy - cmd.energy_cost);
        match cmd.stat.as_str() {
            "str" => pet.str_ += cmd.stat_gain,
            "vit" => pet.vit += cmd.stat_gain,
            "agi" => pet.agi += cmd.stat_gain,
            _ => return Err(DomainError::ValidationError("stat invalide (str|vit|agi)".into())),
        }
        pet.set_cooldown("train", now);
        let saved = self.repo.save(&pet).await?;
        let _ = self
            .repo
            .add_event(saved.id, "train", &format!("Entrainement : +{} {}", cmd.stat_gain, cmd.stat))
            .await;
        Ok(saved)
    }

    async fn visit(&self, cmd: VisitCommand) -> Result<VisitResult, DomainError> {
        if cmd.visitor_id == cmd.target_id {
            return Err(DomainError::ValidationError("tu ne peux pas te visiter toi-meme".into()));
        }
        // Compagnon du visiteur (pour cooldown + limite/jour).
        let mut visitor = self
            .repo
            .get_by_owner(&cmd.guild_id, &cmd.visitor_id)
            .await?
            .ok_or_else(|| DomainError::Conflict("tu n'as pas de compagnon".into()))?;

        let now = Utc::now();
        let remaining = visitor.cooldown_remaining_secs("visit", now, cmd.cooldown_secs);
        if remaining > 0 {
            return Err(DomainError::Conflict(format!(
                "visite en cooldown ({remaining}s restantes)"
            )));
        }
        let today = now.format("%Y-%m-%d").to_string();
        if cmd.max_per_day > 0 && visitor.daily_counter("visit", &today) >= cmd.max_per_day {
            return Err(DomainError::Conflict(format!(
                "limite de {} visites par jour atteinte",
                cmd.max_per_day
            )));
        }

        // Compagnon du visite.
        let mut target = self
            .repo
            .get_by_owner(&cmd.guild_id, &cmd.target_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("ce joueur n'a pas de compagnon".into()))?;
        if target.status == Health::Dead {
            return Err(DomainError::Conflict("le compagnon de ce joueur est mort".into()));
        }

        // Recompense le visite : coins (wallet) + XP.
        if cmd.coins_reward > 0 {
            self.wallet
                .credit(&cmd.guild_id, &cmd.target_id, cmd.coins_reward, "tamagotchi", "Visite recue")
                .await?;
        }
        if cmd.xp_reward > 0 {
            target.xp += cmd.xp_reward;
            target.refresh_level();
            self.repo.save(&target).await?;
        }
        let _ = self
            .repo
            .add_event(
                target.id,
                "visit",
                &format!(
                    "{} a recu une visite de {} (+{} XP +{} coins)",
                    target.name, cmd.visitor_name, cmd.xp_reward, cmd.coins_reward
                ),
            )
            .await;

        // Cooldown + compteur cote visiteur.
        visitor.set_cooldown("visit", now);
        visitor.bump_daily_counter("visit", &today);
        self.repo.save(&visitor).await?;

        Ok(VisitResult {
            target_name: target.name,
            xp_reward: cmd.xp_reward,
            coins_reward: cmd.coins_reward,
        })
    }

    async fn combat(&self, cmd: CombatCommand) -> Result<CombatResult, DomainError> {
        if cmd.attacker_id == cmd.target_id {
            return Err(DomainError::ValidationError("tu ne peux pas te combattre toi-meme".into()));
        }
        let mut att = self
            .repo
            .get_by_owner(&cmd.guild_id, &cmd.attacker_id)
            .await?
            .ok_or_else(|| DomainError::Conflict("tu n'as pas de compagnon".into()))?;
        if att.status == Health::Dead {
            return Err(DomainError::Conflict("ton compagnon est mort".into()));
        }
        let now = Utc::now();
        let remaining = att.cooldown_remaining_secs("combat", now, cmd.cooldown_secs);
        if remaining > 0 {
            return Err(DomainError::Conflict(format!("combat en cooldown ({remaining}s restantes)")));
        }
        if att.energy < cmd.energy_cost {
            return Err(DomainError::Conflict("ton compagnon est epuise".into()));
        }
        let mut def = self
            .repo
            .get_by_owner(&cmd.guild_id, &cmd.target_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("ce joueur n'a pas de compagnon".into()))?;
        if def.status == Health::Dead {
            return Err(DomainError::Conflict("le compagnon de ce joueur est mort".into()));
        }

        // Rolls (RNG genere avant tout await, non maintenu a travers).
        let (roll_a, roll_d) = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let max = cmd.random_max.max(0);
            let hi = if max == 0 { 0 } else { rng.gen_range(0..=max) };
            let lo = if max == 0 { 0 } else { rng.gen_range(0..=max) };
            (hi, lo)
        };
        let power_a = combat_power(att.str_, att.vit, att.agi, cmd.w_str, cmd.w_vit, cmd.w_agi, roll_a);
        let power_d = combat_power(def.str_, def.vit, def.agi, cmd.w_str, cmd.w_vit, cmd.w_agi, roll_d);
        let attacker_won = power_a >= power_d;

        att.energy = clamp_gauge(att.energy - cmd.energy_cost);
        let old_att_elo = att.elo;

        if attacker_won {
            let (nw, nl) = elo_update(att.elo, def.elo, cmd.elo_k);
            att.elo = nw;
            def.elo = nl;
            att.wins += 1;
            def.losses += 1;
            att.xp += cmd.xp_win;
            def.xp += cmd.xp_loss;
        } else {
            let (nw, nl) = elo_update(def.elo, att.elo, cmd.elo_k);
            def.elo = nw;
            att.elo = nl;
            att.losses += 1;
            def.wins += 1;
            att.xp += cmd.xp_loss;
            def.xp += cmd.xp_win;
        }
        att.refresh_level();
        def.refresh_level();
        att.set_cooldown("combat", now);

        self.repo.save(&att).await?;
        self.repo.save(&def).await?;
        let verb = if attacker_won { "a battu" } else { "a perdu contre" };
        let _ = self.repo.add_event(att.id, "combat", &format!("{} {} {}", att.name, verb, def.name)).await;
        let _ = self
            .repo
            .add_event(def.id, "combat", &format!("{} a affronte {}", def.name, cmd.attacker_name))
            .await;

        Ok(CombatResult {
            attacker_won,
            attacker_power: power_a,
            defender_power: power_d,
            defender_name: def.name,
            attacker_new_elo: att.elo,
            attacker_elo_delta: att.elo - old_att_elo,
        })
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
