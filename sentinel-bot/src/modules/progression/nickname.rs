//! Gestion du prefixe `[NN]Pseudo` reflete le niveau global du membre.
//!
//! Applique au level-up global (cf. on_message / on_voice_state_update)
//! et lors du resync admin (Phase 3).

use serenity::all::{Context, EditMember, GuildId, UserId};
use tracing::warn;

/// Limite Discord pour les nicknames de membres.
const DISCORD_NICKNAME_MAX: usize = 32;

/// Retire un eventuel prefixe `[NN]` en debut de chaine.
/// `[12]Darkponey` -> `Darkponey`, `Darkponey` -> `Darkponey`.
pub fn strip_level_prefix(name: &str) -> &str {
    let bytes = name.as_bytes();
    if bytes.first() != Some(&b'[') {
        return name;
    }
    let mut i = 1;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Au moins 1 chiffre + le crochet fermant.
    if i == 1 || i >= bytes.len() || bytes[i] != b']' {
        return name;
    }
    &name[i + 1..]
}

/// Construit `[level]base` en tronquant `base` pour respecter la limite
/// Discord de 32 caracteres.
pub fn build_nickname(base: &str, level: i32) -> String {
    let prefix = format!("[{level}]");
    let prefix_len = prefix.chars().count();
    let max_base = DISCORD_NICKNAME_MAX.saturating_sub(prefix_len);
    let base_truncated: String = base.chars().take(max_base).collect();
    format!("{prefix}{base_truncated}")
}

/// Resultat d'une tentative de renommage. Permet a la commande resync de
/// produire un bilan precis.
#[derive(Debug)]
pub enum ResyncOutcome {
    /// Le pseudo a effectivement ete modifie.
    Renamed,
    /// Le prefixe attendu etait deja en place — rien a faire.
    AlreadyOk,
    /// Cas non actionnable : owner du serveur (Discord interdit), member
    /// introuvable, etc. — pas une erreur, juste un skip silencieux.
    Skipped,
    /// Echec Discord (perms manquantes, rate limit) — message inclus.
    Error(String),
}

/// Renomme le membre en `[level]<base>` ou `base` est l'actuel display_name
/// debarrasse d'un eventuel ancien prefixe `[NN]`.
///
/// Best-effort : log + ignore en cas d'echec. Retourne un `ResyncOutcome`
/// pour permettre a la commande resync d'agreger un bilan.
pub async fn apply_level_prefix(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    level: i32,
) -> ResyncOutcome {
    // Owner du serveur : Discord refuse `Modify Nicknames` sur l'owner.
    let is_owner = ctx
        .cache
        .guild(guild_id)
        .map(|g| g.owner_id == user_id)
        .unwrap_or(false);
    if is_owner {
        return ResyncOutcome::Skipped;
    }

    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, %user_id, "nickname: echec fetch member");
            return ResyncOutcome::Skipped;
        }
    };

    // Base = ce qui est REELLEMENT affiche, pour ne faire qu'ajouter le
    // prefixe sans ecraser le nom du membre :
    //   1. pseudo serveur (`nick`) s'il existe,
    //   2. sinon le nom d'affichage global Discord (`global_name`),
    //   3. sinon le nom de compte (`name`).
    // Avant, on sautait `global_name` -> les membres sans pseudo serveur
    // voyaient leur nom affiche remplace par leur @username brut.
    let current = member
        .nick
        .clone()
        .or_else(|| member.user.global_name.clone())
        .unwrap_or_else(|| member.user.name.clone());
    let base = strip_level_prefix(&current);
    let new_nick = build_nickname(base, level);

    if new_nick == current {
        return ResyncOutcome::AlreadyOk;
    }

    match guild_id
        .edit_member(&ctx.http, user_id, EditMember::new().nickname(&new_nick))
        .await
    {
        Ok(_) => ResyncOutcome::Renamed,
        Err(e) => {
            warn!(error = %e, %user_id, new_nick, "nickname: echec rename");
            ResyncOutcome::Error(e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_no_prefix() {
        assert_eq!(strip_level_prefix("Darkponey"), "Darkponey");
    }

    #[test]
    fn strip_single_digit() {
        assert_eq!(strip_level_prefix("[1]Darkponey"), "Darkponey");
    }

    #[test]
    fn strip_multi_digit() {
        assert_eq!(strip_level_prefix("[123]Darkponey"), "Darkponey");
    }

    #[test]
    fn strip_empty_brackets_kept() {
        // [] sans chiffre n'est pas un prefixe de niveau, on ne touche pas.
        assert_eq!(strip_level_prefix("[]Darkponey"), "[]Darkponey");
    }

    #[test]
    fn strip_bracket_without_close_kept() {
        assert_eq!(strip_level_prefix("[12Darkponey"), "[12Darkponey");
    }

    #[test]
    fn strip_only_prefix() {
        assert_eq!(strip_level_prefix("[5]"), "");
    }

    #[test]
    fn build_short_name() {
        assert_eq!(build_nickname("Alice", 12), "[12]Alice");
    }

    #[test]
    fn build_truncates_long_base() {
        let base = "a".repeat(40);
        let n = build_nickname(&base, 5);
        assert_eq!(n.chars().count(), DISCORD_NICKNAME_MAX);
        assert!(n.starts_with("[5]"));
    }

    #[test]
    fn build_large_level() {
        let n = build_nickname("Bob", 999);
        assert_eq!(n, "[999]Bob");
    }
}
