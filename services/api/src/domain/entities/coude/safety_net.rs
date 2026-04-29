//! Filet de securite coins (cf. COUPE_AMELIORATIONS section 4.4).
//!
//! Quand le solde d un joueur tombe sous 50c, il entre en "phase de
//! recuperation" pendant 3 jours :
//! - Toutes ses pertes sont divisees par 2
//! - Ses paris gagnants sont multiplies par 1.5
//! - Un message quotidien lui rappelle l etat
//!
//! Logique pure ici. La persistance vit dans `coude_safety_nets` et est
//! geree par le service `ManageCoudeSafetyNetService`.

use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;

/// Seuil de declenchement (en coins). Si le wallet tombe sous ce
/// montant, le filet s active.
pub const SAFETY_NET_TRIGGER_COINS: i64 = 50;

/// Duree d activation (en heures, 3 jours).
pub const SAFETY_NET_DURATION_HOURS: i64 = 72;

/// Multiplicateur applique aux pertes pendant la phase (0.5 = pertes /2).
pub const SAFETY_NET_LOSS_MULTIPLIER: f64 = 0.5;

/// Multiplicateur applique aux gains de paris pendant la phase
/// (1.5 = paris gagnants +50%).
pub const SAFETY_NET_BET_GAIN_MULTIPLIER: f64 = 1.5;

/// Snapshot d un filet actif lu en DB.
#[derive(Debug, Clone)]
pub struct ActiveSafetyNet {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub activated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl ActiveSafetyNet {
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.expires_at > now
    }
}

/// Doit-on declencher le filet pour un joueur dont le solde vient de
/// tomber a `current_balance` ?
pub fn should_trigger(current_balance: i64) -> bool {
    current_balance < SAFETY_NET_TRIGGER_COINS
}

/// Reduction d une perte sous filet actif. Floor a 0 pour eviter les
/// pertes negatives sur des arrondis.
pub fn reduce_loss(nominal_loss: i64, has_active_net: bool) -> i64 {
    reduce_loss_with_multiplier(nominal_loss, has_active_net, SAFETY_NET_LOSS_MULTIPLIER)
}

pub fn reduce_loss_with_multiplier(
    nominal_loss: i64,
    has_active_net: bool,
    multiplier: f64,
) -> i64 {
    if !has_active_net || nominal_loss <= 0 {
        return nominal_loss;
    }
    // round() (et non as i64) pour rester coherent avec lucky_shield et
    // eviter +/- 1c systematique selon la parite (3 * 0.5 = 1.5 -> 2, pas 1).
    ((nominal_loss as f64) * multiplier).round() as i64
}

/// Boost d un gain de pari sous filet actif.
pub fn boost_bet_gain(nominal_gain: i64, has_active_net: bool) -> i64 {
    boost_bet_gain_with_multiplier(nominal_gain, has_active_net, SAFETY_NET_BET_GAIN_MULTIPLIER)
}

pub fn boost_bet_gain_with_multiplier(
    nominal_gain: i64,
    has_active_net: bool,
    multiplier: f64,
) -> i64 {
    if !has_active_net || nominal_gain <= 0 {
        return nominal_gain;
    }
    // round() pour rester coherent avec lucky_shield et eviter de spolier
    // le joueur de 0.5c systematique (5 * 1.5 = 7.5 -> 8, pas 7).
    ((nominal_gain as f64) * multiplier).round() as i64
}

#[cfg(test)]
#[path = "tests/safety_net.rs"]
mod tests;
