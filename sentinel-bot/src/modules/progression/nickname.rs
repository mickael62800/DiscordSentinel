//! Gestion des prefixes de pseudo du module progression.
//!
//! Un pseudo peut porter DEUX prefixes en tete : un emoji optionnel selon le
//! role staff le plus eleve du membre, puis le prefixe de niveau `[NN]`, ex.
//! `👑[12]Alice`. Les deux parties sont recomputees ensemble a chaque
//! application pour se preserver mutuellement.
//!
//! Applique au level-up global (cf. on_message / on_voice_state_update), au
//! changement de role (guild_member_update) et lors du resync admin.

use std::collections::HashMap;

use serenity::all::{Context, EditMember, GuildId, UserId};
use serenity::model::guild::Member;
use tracing::warn;

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

use super::{StatsApiKey, MODULE_BOT_NAME};

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

/// Extrait le niveau d'un prefixe `[NN]` en tete, si present.
/// `[12]Alice` -> `Some(12)`, `Alice` -> `None`.
pub fn parse_level_prefix(name: &str) -> Option<i32> {
    let bytes = name.as_bytes();
    if bytes.first() != Some(&b'[') {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 1 || i >= bytes.len() || bytes[i] != b']' {
        return None;
    }
    name[1..i].parse().ok()
}

/// Parse tolerant de la config `staff_role_emojis` au format
/// `role_id:emoji,role_id:emoji`. Les entrees malformees sont ignorees.
/// Emojis unicode uniquement (on coupe au premier `:`).
pub fn parse_role_emojis(csv: &str) -> Vec<(u64, String)> {
    csv.split(',')
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }
            let (id, emoji) = entry.split_once(':')?;
            let id = id.trim().parse::<u64>().ok()?;
            let emoji = emoji.trim();
            if emoji.is_empty() {
                return None;
            }
            Some((id, emoji.to_string()))
        })
        .collect()
}

/// Selectionne l'emoji du role staff le plus eleve du membre.
///
/// `member_roles` : (role_id, position) des roles du membre.
/// `mappings` : table role_id -> emoji issue de `parse_role_emojis`.
/// Retourne l'emoji du role mappe ayant la plus grande `position`, ou `None`
/// si aucun role du membre n'est mappe.
pub fn pick_emoji(member_roles: &[(u64, i64)], mappings: &[(u64, String)]) -> Option<String> {
    let mut best: Option<(i64, &str)> = None;
    for (rid, pos) in member_roles {
        if let Some((_, emoji)) = mappings.iter().find(|(mid, _)| mid == rid) {
            if best.map(|(bp, _)| *pos > bp).unwrap_or(true) {
                best = Some((*pos, emoji.as_str()));
            }
        }
    }
    best.map(|(_, e)| e.to_string())
}

/// Retire les prefixes connus (emoji staff puis niveau `[NN]`) pour retrouver
/// le nom de base. Robuste a l'ordre pour rester idempotent sur re-application.
/// L'emoji est matche comme prefixe exact (gere le multi-codepoint) ; un
/// espace de separation eventuel est aussi retire.
pub fn strip_all_prefixes<'a>(name: &'a str, known_emojis: &[&str]) -> &'a str {
    let mut s = name;
    for e in known_emojis {
        if e.is_empty() {
            continue;
        }
        if let Some(rest) = s.strip_prefix(e) {
            s = rest.strip_prefix(' ').unwrap_or(rest);
            break;
        }
    }
    strip_level_prefix(s)
}

/// Compose `{emoji}{[level]}{base}` en tronquant `base` (par caracteres) pour
/// respecter la limite Discord de 32. Les prefixes sont prioritaires : si
/// emoji+`[level]` atteint deja >= 32, `base` est vide.
pub fn build_nickname_full(base: &str, level: Option<i32>, emoji: Option<&str>) -> String {
    let mut prefix = String::new();
    if let Some(e) = emoji {
        prefix.push_str(e);
    }
    if let Some(l) = level {
        prefix.push_str(&format!("[{l}]"));
    }
    let prefix_len = prefix.chars().count();
    let max_base = DISCORD_NICKNAME_MAX.saturating_sub(prefix_len);
    let base_truncated: String = base.chars().take(max_base).collect();
    format!("{prefix}{base_truncated}")
}

/// Construit `[level]base` en tronquant `base` pour respecter la limite
/// Discord de 32 caracteres. (Cas historique sans emoji, conserve pour
/// compatibilite ; l'apply passe desormais par `build_nickname_full`.)
#[allow(dead_code)]
pub fn build_nickname(base: &str, level: i32) -> String {
    build_nickname_full(base, Some(level), None)
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

/// Charge la config guild du module progression (best-effort, defaut vide).
async fn load_guild_config(ctx: &Context, guild_id: GuildId) -> HashMap<String, String> {
    let base = ctx.data.read().await.get::<ApiClientKey>().cloned();
    match base {
        Some(base) => base
            .get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME)
            .await
            .unwrap_or_default(),
        None => HashMap::new(),
    }
}

/// Point d'entree historique (level-up + resync) : applique le prefixe de
/// niveau `[level]` en preservant aussi l'emoji staff eventuel.
pub async fn apply_level_prefix(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    level: i32,
) -> ResyncOutcome {
    apply_prefixes(ctx, guild_id, user_id, Some(level)).await
}

/// Recompute le pseudo complet `{emoji}{[level]}{base}` a partir de l'etat
/// courant du membre et de la config guild, puis le met a jour si besoin.
///
/// - `level` : niveau a stamper, ou `None` pour ne pas (re)poser de `[NN]`.
/// - L'emoji staff est calcule depuis les roles du membre + la config.
/// - Owner du serveur ignore (Discord refuse le rename de l'owner).
///
/// Best-effort : log + ignore en cas d'echec. Retourne un `ResyncOutcome`.
pub async fn apply_prefixes(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    level: Option<i32>,
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

    let guild_config = load_guild_config(ctx, guild_id).await;
    let staff_enabled = BaseApiClient::config_bool(&guild_config, "staff_prefix_enabled", false);
    let mappings = if staff_enabled {
        parse_role_emojis(&BaseApiClient::config_or(
            &guild_config,
            "staff_role_emojis",
            "",
        ))
    } else {
        Vec::new()
    };

    // Emoji = celui du role mappe le plus haut (par position). Necessite les
    // positions des roles depuis le cache guild ; cache manquant -> pas d'emoji.
    let emoji: Option<String> = if !mappings.is_empty() {
        let positions: Vec<(u64, i64)> = ctx
            .cache
            .guild(guild_id)
            .map(|g| {
                member
                    .roles
                    .iter()
                    .filter_map(|rid| g.roles.get(rid).map(|r| (rid.get(), r.position as i64)))
                    .collect()
            })
            .unwrap_or_default();
        pick_emoji(&positions, &mappings)
    } else {
        None
    };

    // Base = ce qui est REELLEMENT affiche, pour ne faire qu'ajouter les
    // prefixes sans ecraser le nom du membre :
    //   1. pseudo serveur (`nick`) s'il existe,
    //   2. sinon le nom d'affichage global Discord (`global_name`),
    //   3. sinon le nom de compte (`name`).
    let current = member
        .nick
        .clone()
        .or_else(|| member.user.global_name.clone())
        .unwrap_or_else(|| member.user.name.clone());
    let known: Vec<&str> = mappings.iter().map(|(_, e)| e.as_str()).collect();
    let base = strip_all_prefixes(&current, &known);
    let new_nick = build_nickname_full(base, level, emoji.as_deref());

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

/// Declencheur changement de role (guild_member_update) : quand
/// `staff_prefix_enabled`, recompute le pseudo pour refleter le role staff
/// courant, en preservant le prefixe de niveau `[NN]`.
///
/// Le niveau est recupere via l'API ; en cas d'echec, on retombe sur le `[NN]`
/// deja present dans le pseudo pour ne pas le perdre.
pub async fn on_member_update(ctx: &Context, member: &Member) {
    let guild_id = member.guild_id;
    let guild_config = load_guild_config(ctx, guild_id).await;
    if !BaseApiClient::config_bool(&guild_config, "staff_prefix_enabled", false) {
        return;
    }

    let user_id = member.user.id;
    let level: Option<i32> = {
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<StatsApiKey>() {
            match api
                .get_user_level(&guild_id.to_string(), &user_id.to_string())
                .await
            {
                Ok(Some(u)) => Some(u.level),
                _ => member.nick.as_deref().and_then(parse_level_prefix),
            }
        } else {
            member.nick.as_deref().and_then(parse_level_prefix)
        }
    };

    apply_prefixes(ctx, guild_id, user_id, level).await;
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

    // ── parse_level_prefix ──

    #[test]
    fn parse_level_ok() {
        assert_eq!(parse_level_prefix("[12]Alice"), Some(12));
        assert_eq!(parse_level_prefix("[7]"), Some(7));
    }

    #[test]
    fn parse_level_none() {
        assert_eq!(parse_level_prefix("Alice"), None);
        assert_eq!(parse_level_prefix("[]Alice"), None);
        assert_eq!(parse_level_prefix("[12Alice"), None);
    }

    // ── parse_role_emojis ──

    #[test]
    fn parse_emojis_basic() {
        let m = parse_role_emojis("111:\u{1f451},222:\u{1f6e1}\u{fe0f},333:\u{2694}\u{fe0f}");
        assert_eq!(
            m,
            vec![
                (111, "\u{1f451}".to_string()),
                (222, "\u{1f6e1}\u{fe0f}".to_string()),
                (333, "\u{2694}\u{fe0f}".to_string()),
            ]
        );
    }

    #[test]
    fn parse_emojis_skips_malformed() {
        // entree sans `:`, id non numerique, emoji vide, espaces -> ignorees.
        let m = parse_role_emojis("nope, 111 : \u{1f451} ,abc:x,222:,333:\u{2694}");
        assert_eq!(
            m,
            vec![
                (111, "\u{1f451}".to_string()),
                (333, "\u{2694}".to_string())
            ]
        );
    }

    #[test]
    fn parse_emojis_empty() {
        assert!(parse_role_emojis("").is_empty());
    }

    // ── pick_emoji : priorite au role le plus haut ──

    #[test]
    fn pick_highest_role_wins() {
        let mappings = vec![
            (111, "\u{1f451}".to_string()), // pos 30
            (222, "\u{1f6e1}".to_string()), // pos 10
        ];
        let roles = vec![(222, 10i64), (111, 30i64), (999, 5i64)];
        assert_eq!(pick_emoji(&roles, &mappings), Some("\u{1f451}".to_string()));
    }

    #[test]
    fn pick_none_when_no_mapped_role() {
        let mappings = vec![(111, "\u{1f451}".to_string())];
        let roles = vec![(999, 50i64)];
        assert_eq!(pick_emoji(&roles, &mappings), None);
    }

    // ── strip_all_prefixes : idempotence / ordre ──

    #[test]
    fn strip_all_emoji_then_level() {
        let known = ["\u{1f451}"];
        assert_eq!(strip_all_prefixes("\u{1f451}[12]Alice", &known), "Alice");
    }

    #[test]
    fn strip_all_emoji_with_space() {
        let known = ["\u{1f451}"];
        assert_eq!(strip_all_prefixes("\u{1f451} [12]Alice", &known), "Alice");
    }

    #[test]
    fn strip_all_multi_codepoint_emoji() {
        let known = ["\u{1f6e1}\u{fe0f}"];
        assert_eq!(strip_all_prefixes("\u{1f6e1}\u{fe0f}[3]Bob", &known), "Bob");
    }

    #[test]
    fn strip_all_no_prefix() {
        let known = ["\u{1f451}"];
        assert_eq!(strip_all_prefixes("Alice", &known), "Alice");
    }

    #[test]
    fn strip_all_level_only() {
        let known: [&str; 0] = [];
        assert_eq!(strip_all_prefixes("[42]Alice", &known), "Alice");
    }

    // ── build_nickname_full ──

    #[test]
    fn full_emoji_and_level() {
        assert_eq!(
            build_nickname_full("Alice", Some(12), Some("\u{1f451}")),
            "\u{1f451}[12]Alice"
        );
    }

    #[test]
    fn full_no_emoji_no_level_is_base() {
        assert_eq!(build_nickname_full("Alice", None, None), "Alice");
    }

    #[test]
    fn full_level_only_matches_legacy() {
        // byte-for-byte identique a build_nickname (comportement historique).
        assert_eq!(
            build_nickname_full("Alice", Some(5), None),
            build_nickname("Alice", 5)
        );
    }

    #[test]
    fn full_truncates_base_with_emoji() {
        let base = "a".repeat(40);
        let n = build_nickname_full(&base, Some(5), Some("\u{1f451}"));
        assert_eq!(n.chars().count(), DISCORD_NICKNAME_MAX);
        assert!(n.starts_with("\u{1f451}[5]"));
    }

    #[test]
    fn full_prefix_priority_drops_base() {
        // emoji + niveau enorme : le prefixe est conserve, base tronquee.
        let n = build_nickname_full("Alice", Some(1234567890), Some("\u{1f451}"));
        assert!(n.starts_with("\u{1f451}[1234567890]"));
    }

    #[test]
    fn full_reapply_idempotent() {
        let known = ["\u{1f451}"];
        let first = build_nickname_full("Alice", Some(12), Some("\u{1f451}"));
        let base = strip_all_prefixes(&first, &known);
        let second = build_nickname_full(base, Some(12), Some("\u{1f451}"));
        assert_eq!(first, second);
    }
}
