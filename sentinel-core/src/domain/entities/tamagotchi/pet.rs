//! Entite compagnon + logique pure de cycle de vie (decroissance des
//! jauges, maladie, mort, guerison par le soin).

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Etat de sante du compagnon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Sick,
    Dead,
}

impl Health {
    pub fn as_str(&self) -> &'static str {
        match self {
            Health::Healthy => "healthy",
            Health::Sick => "sick",
            Health::Dead => "dead",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "sick" => Health::Sick,
            "dead" => Health::Dead,
            _ => Health::Healthy,
        }
    }
}

/// Compagnon (mappe sur la table `pets`).
#[derive(Debug, Clone)]
pub struct Pet {
    pub id: Uuid,
    pub guild_id: String,
    pub owner_id: String,
    pub name: String,
    pub species: String,
    pub specialization: Option<String>,

    pub level: i32,
    pub xp: i64,
    pub born_at: DateTime<Utc>,

    pub hunger: i32,
    pub happiness: i32,
    pub energy: i32,

    pub status: Health,
    pub hunger_zero_since: Option<DateTime<Utc>>,
    pub sick_since: Option<DateTime<Utc>>,
    pub died_at: Option<DateTime<Utc>>,

    pub str_: i32,
    pub vit: i32,
    pub agi: i32,
    pub stat_points: i32,

    pub elo: i32,
    pub wins: i32,
    pub losses: i32,

    /// Cooldowns par action : objet JSON { "feed": "<rfc3339>", ... }.
    pub cooldowns: serde_json::Value,

    pub last_decay_at: DateTime<Utc>,
}

impl Pet {
    /// Secondes restantes avant que `action` soit de nouveau disponible.
    /// 0 si pas de cooldown actif.
    pub fn cooldown_remaining_secs(&self, action: &str, now: DateTime<Utc>, cd_secs: i64) -> i64 {
        if cd_secs <= 0 {
            return 0;
        }
        let last = self
            .cooldowns
            .get(action)
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc));
        match last {
            Some(last) => {
                let elapsed = now.signed_duration_since(last).num_seconds();
                (cd_secs - elapsed).max(0)
            }
            None => 0,
        }
    }

    /// Enregistre l'instant `now` comme dernier usage de `action`.
    pub fn set_cooldown(&mut self, action: &str, now: DateTime<Utc>) {
        if !self.cooldowns.is_object() {
            self.cooldowns = serde_json::json!({});
        }
        if let Some(map) = self.cooldowns.as_object_mut() {
            map.insert(action.to_string(), serde_json::Value::String(now.to_rfc3339()));
        }
    }

    /// Recalcule le niveau a partir de l'XP.
    pub fn refresh_level(&mut self) {
        self.level = level_from_xp(self.xp);
    }
}

/// Parametres de cycle de vie (issus de la config guild, en secondes).
#[derive(Debug, Clone, Copy)]
pub struct TickConfig {
    pub hunger_decay_per_hour: i32,
    pub happiness_decay_per_hour: i32,
    pub energy_decay_per_hour: i32,
    /// Faim a 0 pendant ce delai -> maladie.
    pub sick_after_secs: i64,
    /// Malade non soigne pendant ce delai -> mort.
    pub death_after_sick_secs: i64,
    /// Seuil "jauge basse" : si toutes les jauges repassent au-dessus, le
    /// compagnon malade guerit (le soin suffit a guerir, pas de bouton dedie).
    pub low_threshold: i32,
}

fn clamp_gauge(v: i32) -> i32 {
    v.clamp(0, 100)
}

/// Resultat d'un tick (pour que l'appelant sache quoi journaliser/notifier).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickOutcome {
    Unchanged,
    Decayed,
    FellSick,
    Recovered,
    Died,
}

impl Pet {
    /// Applique la decroissance + les transitions de sante depuis
    /// `last_decay_at` jusqu'a `now`. Fonction PURE (mute la struct en
    /// memoire, aucun I/O). Retourne l'evenement le plus notable.
    pub fn apply_tick(&mut self, now: DateTime<Utc>, cfg: &TickConfig) -> TickOutcome {
        if self.status == Health::Dead {
            return TickOutcome::Unchanged;
        }
        let elapsed = now.signed_duration_since(self.last_decay_at).num_seconds();
        if elapsed <= 0 {
            return TickOutcome::Unchanged;
        }
        let hours = elapsed as f64 / 3600.0;

        let before = (self.hunger, self.happiness, self.energy, self.status);

        self.hunger = clamp_gauge(self.hunger - (cfg.hunger_decay_per_hour as f64 * hours).round() as i32);
        self.happiness = clamp_gauge(self.happiness - (cfg.happiness_decay_per_hour as f64 * hours).round() as i32);
        self.energy = clamp_gauge(self.energy - (cfg.energy_decay_per_hour as f64 * hours).round() as i32);

        // Suivi "faim a 0".
        if self.hunger == 0 {
            if self.hunger_zero_since.is_none() {
                self.hunger_zero_since = Some(now);
            }
        } else {
            self.hunger_zero_since = None;
        }

        self.last_decay_at = now;

        // Transitions de sante.
        let mut outcome = if before.0 != self.hunger || before.1 != self.happiness || before.2 != self.energy {
            TickOutcome::Decayed
        } else {
            TickOutcome::Unchanged
        };

        match self.status {
            Health::Healthy => {
                if let Some(zero) = self.hunger_zero_since {
                    if now.signed_duration_since(zero).num_seconds() >= cfg.sick_after_secs {
                        self.status = Health::Sick;
                        self.sick_since = Some(now);
                        outcome = TickOutcome::FellSick;
                    }
                }
            }
            Health::Sick => {
                // Soin suffisant -> guerison.
                if self.hunger > cfg.low_threshold
                    && self.happiness > cfg.low_threshold
                    && self.energy > cfg.low_threshold
                {
                    self.status = Health::Healthy;
                    self.sick_since = None;
                    outcome = TickOutcome::Recovered;
                } else if let Some(since) = self.sick_since {
                    if now.signed_duration_since(since).num_seconds() >= cfg.death_after_sick_secs {
                        self.status = Health::Dead;
                        self.died_at = Some(now);
                        outcome = TickOutcome::Died;
                    }
                }
            }
            Health::Dead => {}
        }

        outcome
    }
}

/// Donnees de creation d'un compagnon.
#[derive(Debug, Clone)]
pub struct NewPet {
    pub guild_id: String,
    pub owner_id: String,
    pub name: String,
    pub species: String,
    pub str_: i32,
    pub vit: i32,
    pub agi: i32,
}

/// Entree du journal d'actions (carte "Dernieres actions").
#[derive(Debug, Clone)]
pub struct PetEvent {
    pub kind: String,
    pub detail: String,
    pub created_at: DateTime<Utc>,
}

/// Niveau atteint pour un XP cumule donne. Courbe : il faut
/// `100 * n` XP pour passer du niveau n au niveau n+1 (cumulatif).
pub fn level_from_xp(xp: i64) -> i32 {
    let mut level = 1i32;
    let mut remaining = xp.max(0);
    loop {
        let needed = xp_for_next(level);
        if remaining < needed {
            break;
        }
        remaining -= needed;
        level += 1;
    }
    level
}

/// XP necessaire pour passer du niveau `level` au suivant.
pub fn xp_for_next(level: i32) -> i64 {
    100 * level.max(1) as i64
}

/// (xp_dans_le_niveau, xp_pour_le_niveau_suivant) pour l'affichage "179/325".
pub fn xp_progress(xp: i64) -> (i64, i64) {
    let mut remaining = xp.max(0);
    let mut level = 1i32;
    loop {
        let needed = xp_for_next(level);
        if remaining < needed {
            return (remaining, needed);
        }
        remaining -= needed;
        level += 1;
    }
}

#[cfg(test)]
#[path = "tests/pet.rs"]
mod tests;
