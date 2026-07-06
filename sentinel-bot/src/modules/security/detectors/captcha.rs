use std::time::Instant;

use dashmap::DashMap;
use rand::seq::SliceRandom;
use rand::Rng;
use serenity::builder::{CreateActionRow, CreateButton, CreateMessage};
use serenity::model::id::{GuildId, UserId};
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::shared::embeds::info_embed;

/// Identifiant du bouton de verification captcha.
pub const CAPTCHA_BUTTON_ID: &str = "sentinel_captcha_verify";

/// Prefixe des boutons captcha math.
pub const CAPTCHA_MATH_PREFIX: &str = "sentinel_captcha_math_";

/// Stockage des captchas en attente (math).
/// Cle: (guild_id, user_id) -> (index correct du bouton, timestamp).
pub struct CaptchaPending {
    pending: DashMap<(GuildId, UserId), (usize, Instant)>,
    /// Duree de vie d'un captcha en secondes (au-dela l'entree est invalide).
    ttl_secs: u64,
}

#[allow(dead_code)]
impl CaptchaPending {
    pub fn new() -> Self {
        Self::with_ttl(600) // 10 minutes par defaut
    }

    pub fn with_ttl(ttl_secs: u64) -> Self {
        Self {
            pending: DashMap::new(),
            ttl_secs,
        }
    }

    /// Enregistre un captcha math en attente.
    pub fn store(&self, guild_id: GuildId, user_id: UserId, correct_index: usize) {
        self.pending
            .insert((guild_id, user_id), (correct_index, Instant::now()));
    }

    /// Verifie si le bouton presse est correct. Retourne:
    /// - `Some(true/false)` si un captcha valide existe
    /// - `None` si aucun captcha en attente OU si l'entree a expire
    pub fn verify(&self, guild_id: GuildId, user_id: UserId, pressed_index: usize) -> Option<bool> {
        let entry = self.pending.get(&(guild_id, user_id))?;
        let (correct, stored_at) = *entry.value();
        drop(entry);
        if stored_at.elapsed() >= std::time::Duration::from_secs(self.ttl_secs) {
            // Entree expiree : on la supprime et on considere qu'il n'y a plus de captcha.
            self.pending.remove(&(guild_id, user_id));
            return None;
        }
        // USAGE UNIQUE (anti brute-force) : on consomme l'entree que la reponse
        // soit bonne OU mauvaise. Sinon un self-bot cliquerait les 4 boutons et
        // trouverait le bon en <= 4 essais garantis. Une mauvaise reponse
        // invalide donc le captcha (le user devra en obtenir un nouveau).
        self.pending.remove(&(guild_id, user_id));
        Some(pressed_index == correct)
    }

    /// Supprime toutes les entrees expirees. Appele par la task de background.
    pub fn cleanup_expired(&self) {
        let ttl = std::time::Duration::from_secs(self.ttl_secs);
        self.pending.retain(|_, (_, ts)| ts.elapsed() < ttl);
    }

    /// Supprime un captcha en attente (apres verification ou timeout).
    pub fn remove(&self, guild_id: GuildId, user_id: UserId) {
        self.pending.remove(&(guild_id, user_id));
    }

    /// Retourne les captchas expires (pour nettoyage).
    pub fn expired(&self, timeout_secs: u64) -> Vec<(GuildId, UserId)> {
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let now = Instant::now();
        self.pending
            .iter()
            .filter(|entry| now.duration_since(entry.value().1) >= timeout)
            .map(|entry| *entry.key())
            .collect()
    }
}

/// Genere un challenge mathematique.
/// Retourne (question, correct_answer_string, choices_with_correct_index).
pub fn generate_math_challenge() -> (String, usize, Vec<String>) {
    let mut rng = rand::thread_rng();
    let a = rng.gen_range(1..20u32);
    let b = rng.gen_range(1..20u32);
    let correct = a + b;

    let mut choices: Vec<u32> = vec![correct];
    while choices.len() < 4 {
        let wrong = rng.gen_range(2..40u32);
        if !choices.contains(&wrong) {
            choices.push(wrong);
        }
    }

    choices.shuffle(&mut rng);

    let correct_index = choices.iter().position(|&v| v == correct).unwrap();
    let labels: Vec<String> = choices.iter().map(|v| v.to_string()).collect();
    let question = format!("Combien font {} + {} ?", a, b);

    (question, correct_index, labels)
}

/// Envoie un captcha math en DM avec 4 boutons.
pub async fn send_math_challenge(
    ctx: &Context,
    user_id: UserId,
    guild_id: GuildId,
    guild_name: &str,
    pending: &CaptchaPending,
) -> bool {
    let user = match user_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(e) => {
            warn!(error = %e, user_id = %user_id, "Impossible de recuperer l'utilisateur pour captcha math");
            return false;
        }
    };

    let dm_channel = match user.create_dm_channel(&ctx.http).await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, user_id = %user_id, "Impossible de creer le DM pour captcha math");
            return false;
        }
    };

    let (question, correct_index, labels) = generate_math_challenge();

    // Stocker la reponse correcte
    pending.store(guild_id, user_id, correct_index);

    let buttons: Vec<CreateButton> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            CreateButton::new(format!("{}{}", CAPTCHA_MATH_PREFIX, i))
                .label(label)
                .style(serenity::all::ButtonStyle::Primary)
        })
        .collect();

    let row = CreateActionRow::Buttons(buttons);

    let embed = info_embed("\u{1f6e1}\u{fe0f} Verification requise")
        .description(format!(
            "**Verification de securite — {}**\n\n\
             Pour prouver que vous etes humain, repondez a cette question :\n\n\
             **{}**",
            guild_name, question
        ))
        .field(
            "\u{23f1}\u{fe0f}",
            "Vous avez **5 minutes** pour repondre, sinon vous serez expulse.",
            false,
        );

    let message = CreateMessage::new().embed(embed).components(vec![row]);

    match dm_channel.send_message(&ctx.http, message).await {
        Ok(_) => {
            info!(user_id = %user_id, "Challenge captcha math envoye en DM");
            true
        }
        Err(e) => {
            error!(error = %e, user_id = %user_id, "Impossible d'envoyer le captcha math en DM");
            pending.remove(guild_id, user_id);
            false
        }
    }
}

/// Genere un code captcha simple (6 caracteres alphanumeriques).
#[allow(dead_code)]
pub fn generate_code() -> String {
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'A' + idx - 10) as char
            }
        })
        .collect()
}

/// Envoie un message de verification en DM avec un bouton.
/// Le code captcha est encode dans le custom_id du bouton.
pub async fn send_challenge(ctx: &Context, user_id: UserId, guild_name: &str) -> bool {
    let user = match user_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(e) => {
            warn!(error = %e, user_id = %user_id, "Impossible de recuperer l'utilisateur pour captcha");
            return false;
        }
    };

    let dm_channel = match user.create_dm_channel(&ctx.http).await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, user_id = %user_id, "Impossible de creer le DM pour captcha");
            return false;
        }
    };

    let button = CreateButton::new(CAPTCHA_BUTTON_ID)
        .label("Je suis humain — Verifier")
        .style(serenity::all::ButtonStyle::Success);

    let row = CreateActionRow::Buttons(vec![button]);

    let embed = info_embed("\u{1f6e1}\u{fe0f} Verification requise")
        .description(format!(
            "**Verification de securite — {}**\n\n\
             Votre compte a ete detecte comme potentiellement suspect.\n\
             Cliquez sur le bouton ci-dessous pour confirmer que vous etes humain.",
            guild_name
        ))
        .field(
            "\u{23f1}\u{fe0f}",
            "Vous avez **5 minutes** pour vous verifier, sinon vous serez expulse.",
            false,
        );

    let message = CreateMessage::new().embed(embed).components(vec![row]);

    match dm_channel.send_message(&ctx.http, message).await {
        Ok(_) => {
            info!(user_id = %user_id, "Challenge captcha envoye en DM");
            true
        }
        Err(e) => {
            error!(error = %e, user_id = %user_id, "Impossible d'envoyer le captcha en DM");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── generate_math_challenge ──

    #[test]
    fn math_challenge_has_4_choices() {
        let (_, _, choices) = generate_math_challenge();
        assert_eq!(choices.len(), 4);
    }

    #[test]
    fn math_challenge_correct_index_valid() {
        let (_, correct_idx, choices) = generate_math_challenge();
        assert!(correct_idx < choices.len());
    }

    #[test]
    fn math_challenge_correct_answer_is_sum() {
        for _ in 0..20 {
            let (question, correct_idx, choices) = generate_math_challenge();
            let parts: Vec<&str> = question.split_whitespace().collect();
            let a: u32 = parts[2].parse().unwrap();
            let b: u32 = parts[4].parse().unwrap();
            let expected = (a + b).to_string();
            assert_eq!(choices[correct_idx], expected);
        }
    }

    #[test]
    fn math_challenge_choices_are_distinct() {
        for _ in 0..20 {
            let (_, _, choices) = generate_math_challenge();
            let mut sorted = choices.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), 4);
        }
    }

    // ── CaptchaPending ──

    #[test]
    fn pending_store_and_verify_correct() {
        let pending = CaptchaPending::new();
        let guild = GuildId::new(1);
        let user = UserId::new(42);
        pending.store(guild, user, 2);
        assert_eq!(pending.verify(guild, user, 2), Some(true));
    }

    #[test]
    fn pending_verify_wrong() {
        let pending = CaptchaPending::new();
        let guild = GuildId::new(1);
        let user = UserId::new(42);
        pending.store(guild, user, 2);
        // Usage unique : la 1re mauvaise reponse consomme le captcha ...
        assert_eq!(pending.verify(guild, user, 0), Some(false));
        // ... les tentatives suivantes n'ont plus de captcha (anti brute-force).
        assert_eq!(pending.verify(guild, user, 1), None);
        assert_eq!(pending.verify(guild, user, 3), None);
    }

    #[test]
    fn pending_verify_no_entry() {
        let pending = CaptchaPending::new();
        let guild = GuildId::new(1);
        let user = UserId::new(42);
        assert_eq!(pending.verify(guild, user, 0), None);
    }

    #[test]
    fn pending_remove() {
        let pending = CaptchaPending::new();
        let guild = GuildId::new(1);
        let user = UserId::new(42);
        pending.store(guild, user, 1);
        pending.remove(guild, user);
        assert_eq!(pending.verify(guild, user, 1), None);
    }

    #[test]
    fn pending_expired() {
        let pending = CaptchaPending::new();
        let guild = GuildId::new(1);
        let user = UserId::new(42);
        pending.pending.insert(
            (guild, user),
            (0, Instant::now() - std::time::Duration::from_secs(600)),
        );
        let expired = pending.expired(300);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], (guild, user));
    }

    #[test]
    fn pending_not_expired() {
        let pending = CaptchaPending::new();
        let guild = GuildId::new(1);
        let user = UserId::new(42);
        pending.store(guild, user, 0);
        let expired = pending.expired(300);
        assert!(expired.is_empty());
    }

    // ── generate_code ──

    #[test]
    fn code_has_6_chars() {
        let code = generate_code();
        assert_eq!(code.len(), 6);
    }

    #[test]
    fn code_is_alphanumeric_uppercase() {
        let code = generate_code();
        assert!(
            code.chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()),
            "Code doit etre alphanumerique majuscule, got: {}",
            code
        );
    }

    #[test]
    fn code_generates_different_values() {
        let codes: Vec<String> = (0..10).map(|_| generate_code()).collect();
        // Au moins 2 codes differents sur 10
        let unique: std::collections::HashSet<&String> = codes.iter().collect();
        assert!(
            unique.len() >= 2,
            "generate_code doit produire des valeurs variees"
        );
    }
}
