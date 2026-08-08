use serenity::builder::CreateEmbed;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;

use super::audit_event;
use super::watched_users;
use super::{log, post_to_channel, send_event};

/// Salons de log vocal : cle dediee puis fallback log_channel_id (gere par
/// post_to_channel).
const VOICE_LOG_KEYS: &[&str] = &["voice_log_channel_id"];

pub async fn handle_state_update(ctx: &Context, old: Option<VoiceState>, new: &VoiceState) {
    let gid = match new.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    let user_id = new.user_id.to_string();
    let user_name = new
        .member
        .as_ref()
        .map(|m| m.user.name.clone())
        .unwrap_or_default();

    let old_channel = old.as_ref().and_then(|o| o.channel_id);
    let new_channel = new.channel_id;

    let event_type = match (old_channel, new_channel) {
        (None, Some(_)) => "voice_join",
        (Some(_), None) => "voice_leave",
        (Some(a), Some(b)) if a != b => "voice_move",
        _ => return,
    };

    let voice_msg = match (event_type, old_channel, new_channel) {
        ("voice_join", _, Some(ch)) => format!("{} a rejoint le salon vocal {}", user_name, ch),
        ("voice_leave", Some(ch), _) => format!("{} a quitte le salon vocal {}", user_name, ch),
        ("voice_move", Some(old), Some(new)) => {
            format!("{} a change de salon vocal {} -> {}", user_name, old, new)
        }
        _ => String::new(),
    };
    log(ctx, "info", &gid, &voice_msg).await;

    // Embed Discord (TOUS les salons vocaux, pas seulement les temporaires) :
    // entree = vert + fleche droite, sortie = rouge + fleche gauche, deplacement
    // = bleu. Poste dans voice_log_channel_id (fallback log_channel_id).
    let (title, color, line): (&str, u32, String) = match (event_type, old_channel, new_channel) {
        ("voice_join", _, Some(ch)) => (
            "Connexion vocale",
            0x57F287, // vert
            format!(
                "\u{1f7e2}\u{27a1}\u{fe0f} <@{}> a rejoint <#{}>",
                user_id, ch
            ),
        ),
        ("voice_leave", Some(ch), _) => (
            "Deconnexion vocale",
            0xED4245, // rouge
            format!(
                "\u{1f534}\u{2b05}\u{fe0f} <@{}> a quitte <#{}>",
                user_id, ch
            ),
        ),
        ("voice_move", Some(a), Some(b)) => (
            "Changement de vocal",
            0x5865F2, // bleu
            format!(
                "\u{1f504} <@{}> a change : <#{}> \u{2192} <#{}>",
                user_id, a, b
            ),
        ),
        _ => ("Vocal", 0x95A5A6, voice_msg.clone()),
    };
    let embed = CreateEmbed::new()
        .description(line)
        .color(color)
        .footer(serenity::builder::CreateEmbedFooter::new(format!(
            "{title} · {user_name}"
        )))
        .timestamp(serenity::model::Timestamp::now());
    post_to_channel(ctx, &gid, VOICE_LOG_KEYS, embed).await;

    // Noms de salons resolus a l'ecriture. Sans eux, le journal web n'affiche
    // que des identifiants numeriques illisibles — et un salon supprime
    // devient impossible a identifier a posteriori, contrairement a Discord
    // qui rend la mention <#id> tant que le salon existe.
    let from_name = channel_name(ctx, old_channel).await;
    let to_name = channel_name(ctx, new_channel).await;

    let mut evt = audit_event::simple(gid.clone(), event_type)
        .with_actor(&user_id, &user_name)
        .with_details(serde_json::json!({
            "from_channel": old_channel.map(|c| c.to_string()),
            "from_channel_name": from_name,
            "to_channel": new_channel.map(|c| c.to_string()),
            "to_channel_name": to_name,
        }));
    evt.channel_id = new_channel.map(|c| c.to_string());
    evt.channel_name = to_name.clone().or_else(|| from_name.clone());

    send_event(ctx, evt).await;

    // Surveillance
    let channel_str = new_channel.or(old_channel).map(|c| c.to_string());
    watched_users::track_activity(
        ctx, &gid, &user_id, event_type,
        channel_str.as_deref(), None,
        Some(&voice_msg),
        serde_json::json!({"from": old_channel.map(|c| c.to_string()), "to": new_channel.map(|c| c.to_string())}),
    ).await;
}

/// Nom lisible d'un salon vocal. Passe par le cache serenity puis l'API HTTP ;
/// renvoie None plutot que d'echouer, le nom etant un confort d'affichage.
async fn channel_name(
    ctx: &Context,
    channel: Option<serenity::model::id::ChannelId>,
) -> Option<String> {
    let id = channel?;
    match id.to_channel(&ctx).await {
        Ok(ch) => ch.guild().map(|g| g.name().to_string()),
        Err(_) => None,
    }
}
