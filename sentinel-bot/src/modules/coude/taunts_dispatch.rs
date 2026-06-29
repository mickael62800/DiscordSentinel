//! Dispatch des raillerie events vers Discord (Phase 9 Part D).
//!
//! CE FICHIER NE CONTIENT AUCUNE LOGIQUE METIER. Il ne fait que :
//!   1. Poster le `message` du TauntEvent dans le `channel_id` fourni
//!   2. Renommer le membre en appliquant le `nickname_suffix` fourni
//!
//! Tout ce qui est cuisine cote API (catalogue de messages, seuils,
//! selection de suffixe, opt-outs) est deja fait avant que le bot reçoive
//! l'event. Ici on ne fait que de l'IO Discord.

use serenity::all::{
    ChannelId, Context, CreateEmbed, CreateEmbedFooter, CreateMessage, EditMember, GuildId, UserId,
};

use crate::modules::coude::api_client::TauntEvent;

/// Limite Discord sur le nickname : 32 caracteres. On tronque
/// proprement le base_name si l'ajout du suffixe depasse.
const DISCORD_NICKNAME_MAX: usize = 32;

/// Dispatch tous les events d'une response API. Iter chaque event, post
/// + rename. Les erreurs sont loggees et non-fatales (un taunt rate
/// n'empeche pas le jeu).
pub async fn dispatch_all(ctx: &Context, guild_id: GuildId, events: &[TauntEvent]) {
    for ev in events {
        post_taunt_message(ctx, ev).await;
        apply_nickname_suffix(ctx, guild_id, ev).await;
    }
}

async fn post_taunt_message(ctx: &Context, ev: &TauntEvent) {
    // Si le message est vide, messages_enabled=false cote API : on skippe
    // le post sans toucher au rename.
    if ev.message.is_empty() {
        return;
    }
    let channel_id: u64 = match ev.channel_id.parse() {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, channel_id = %ev.channel_id, "channel_id taunt invalide");
            return;
        }
    };
    let channel = ChannelId::new(channel_id);
    let embed = CreateEmbed::new()
        .title("\u{1f525} Raillerie automatique")
        .description(&ev.message)
        .color(match ev.streak_kind.as_str() {
            "win" => 0xF1C40F,          // or
            "loss" => 0xE74C3C,         // rouge
            "steal_victim" => 0x9B59B6, // violet
            _ => 0x95A5A6,
        })
        .footer(CreateEmbedFooter::new(format!(
            "Serie : {} × {}",
            ev.streak_kind, ev.streak_value
        )));
    if let Err(e) = channel
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        tracing::warn!(error = %e, channel_id = %ev.channel_id, "Echec post taunt message");
    }
}

/// Renomme un membre en ajoutant un suffixe (best-effort, log+ignore en
/// cas d echec). Utilise par le branchement Chicken (cf. COUPE_AMELIORATIONS
/// 5.1) pour appliquer " le Poulet" au pseudo de la cible.
pub async fn apply_suffix_to_user(ctx: &Context, guild_id: GuildId, user_id: UserId, suffix: &str) {
    if suffix.is_empty() {
        return;
    }
    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, %user_id, "Echec fetch member pour rename curse");
            return;
        }
    };
    let current = member
        .nick
        .clone()
        .unwrap_or_else(|| member.user.name.clone());
    if current.ends_with(suffix) {
        return;
    }
    let suffix_len = suffix.chars().count();
    let max_base = DISCORD_NICKNAME_MAX.saturating_sub(suffix_len);
    let base: String = current.chars().take(max_base).collect();
    let new_nick = format!("{}{}", base, suffix);
    if let Err(e) = guild_id
        .edit_member(&ctx.http, user_id, EditMember::new().nickname(&new_nick))
        .await
    {
        tracing::warn!(error = %e, %user_id, new_nick, "Echec rename member (curse)");
    }
}

async fn apply_nickname_suffix(ctx: &Context, guild_id: GuildId, ev: &TauntEvent) {
    // Si le suffixe est vide, rename_enabled=false cote API : aucun rename.
    if ev.nickname_suffix.is_empty() {
        return;
    }
    let user_id: u64 = match ev.target_user_id.parse() {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, user_id = %ev.target_user_id, "user_id taunt invalide");
            return;
        }
    };
    let user = UserId::new(user_id);

    // Lit le member pour recuperer le display name courant.
    let member = match guild_id.member(&ctx.http, user).await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, user_id = %ev.target_user_id, "Echec fetch member pour rename");
            return;
        }
    };

    // Nom de base : on part du nickname courant s'il y en a un, sinon
    // du user.name. On retire les anciens suffixes connus pour eviter
    // l'accumulation (" (en feu) (en feu) (KO)"...). Pour garder le bot
    // 100% IO, on ne fait qu'une operation deterministe simple : si le
    // nickname courant finit par le suffixe qu'on veut appliquer, on
    // ne refait rien.
    let current = member
        .nick
        .clone()
        .unwrap_or_else(|| member.user.name.clone());
    if current.ends_with(&ev.nickname_suffix) {
        return;
    }

    // Tronque le base pour que base + suffix tiennent dans 32 chars.
    let suffix_len = ev.nickname_suffix.chars().count();
    let max_base = DISCORD_NICKNAME_MAX.saturating_sub(suffix_len);
    let base: String = current.chars().take(max_base).collect();
    let new_nick = format!("{}{}", base, ev.nickname_suffix);

    if let Err(e) = guild_id
        .edit_member(&ctx.http, user, EditMember::new().nickname(&new_nick))
        .await
    {
        tracing::warn!(error = %e, user_id = %ev.target_user_id, new_nick, "Echec rename member");
    }
}
