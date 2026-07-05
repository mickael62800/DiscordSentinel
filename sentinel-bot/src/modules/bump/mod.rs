//! Bump rewards : detecte un /bump reussi (Disboard ou DiscordL), recompense
//! l'auteur en coins (recompense graduee selon le nombre de bumps de la
//! semaine, calculee cote API) et rappelle quand un nouveau bump est possible
//! apres le cooldown.
//!
//! Multi-provider (DRY) : chaque plateforme de bump est decrite par une entree
//! `BumpProvider` dans le registre `PROVIDERS`. Le chemin
//! record/reward/VIP/annonce est ecrit UNE seule fois, parametre par le
//! provider detecte. Ajouter une plateforme = une entree ici + un jeu de cles
//! de config cote API.

use std::time::Duration;

use serenity::all::MessageInteractionMetadata;
use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, UserId};
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

pub const MODULE_BOT_NAME: &str = "bump-bot";

/// Description d'une plateforme de bump (Disboard, DiscordL, ...).
struct BumpProvider {
    /// User id du bot qui poste la confirmation de /bump.
    bot_id: u64,
    /// Identifiant stable envoye a l'API + namespace de config ("disboard").
    key: &'static str,
    /// Nom lisible pour les annonces / rappels.
    display: &'static str,
    /// Texte de la commande a rappeler (ex: "/bump (Disboard)").
    bump_hint: &'static str,
    /// Cooldown par defaut en minutes (indicatif ; l'API tranche).
    #[allow(dead_code)]
    default_cooldown_min: i64,
    /// Detection d'un bump REUSSI (et pas un cooldown/echec) pour ce provider.
    detect: fn(&Message) -> bool,
}

/// Disboard (bot historique).
const DISBOARD: BumpProvider = BumpProvider {
    bot_id: 302050872383242240,
    key: "disboard",
    display: "Disboard",
    bump_hint: "/bump (Disboard)",
    default_cooldown_min: 120,
    detect: detect_disboard,
};

/// DiscordL (discordl.org).
const DISCORDL: BumpProvider = BumpProvider {
    bot_id: 528557940811104258,
    key: "discordl",
    display: "DiscordL",
    bump_hint: "/bump (DiscordL)",
    default_cooldown_min: 240,
    detect: detect_discordl,
};

/// Registre des plateformes supportees.
static PROVIDERS: &[BumpProvider] = &[DISBOARD, DISCORDL];

fn provider_for_bot(bot_id: u64) -> Option<&'static BumpProvider> {
    PROVIDERS.iter().find(|p| p.bot_id == bot_id)
}

fn provider_by_key(key: &str) -> Option<&'static BumpProvider> {
    PROVIDERS.iter().find(|p| p.key == key)
}

/// `true` si l'embed Disboard indique un bump REUSSI (et pas un cooldown/echec).
fn detect_disboard(msg: &Message) -> bool {
    let mut positive = false;
    for e in &msg.embeds {
        let desc = e.description.as_deref().unwrap_or("").to_lowercase();
        // Echec / cooldown Disboard : "please wait ... minutes", "patienter".
        if desc.contains("minutes") || desc.contains("wait") || desc.contains("patient") {
            return false;
        }
        if desc.contains("done")
            || desc.contains("effectu")
            || desc.contains("👍")
            || desc.contains(":thumbsup:")
        {
            positive = true;
        }
    }
    positive
}

/// `true` si l'embed DiscordL indique un bump REUSSI. Le message de succes a un
/// titre "Résultat du Bump sur DiscordL" et une description avec "✅" + "a BUMP".
fn detect_discordl(msg: &Message) -> bool {
    let mut positive = false;
    for e in &msg.embeds {
        let title = e.title.as_deref().unwrap_or("").to_lowercase();
        let desc = e.description.as_deref().unwrap_or("").to_lowercase();
        // Cooldown / echec : mots generiques d'attente.
        for hay in [&title, &desc] {
            if hay.contains("wait")
                || hay.contains("minutes")
                || hay.contains("patient")
                || hay.contains("attends")
                || hay.contains("prochain")
                || hay.contains("déjà")
            {
                return false;
            }
        }
        if title.contains("résultat du bump") || desc.contains("✅") || desc.contains("a bump") {
            positive = true;
        }
    }
    positive
}

/// Resout l'auteur du /bump : d'abord via interaction_metadata (reponse de
/// commande), sinon en repli via la premiere mention d'un user non-bot dans la
/// description de l'embed (DiscordL mentionne le bumpeur).
fn resolve_bumper(msg: &Message) -> Option<UserId> {
    if let Some(MessageInteractionMetadata::Command(meta)) = msg.interaction_metadata.as_deref() {
        if !meta.user.bot {
            return Some(meta.user.id);
        }
    }
    // Repli : premiere mention non-bot dans l'embed (si presente dans msg.mentions).
    for e in &msg.embeds {
        let desc = e.description.as_deref().unwrap_or("");
        for m in &msg.mentions {
            if !m.bot && desc.contains(&format!("<@{}>", m.id)) {
                return Some(m.id);
            }
        }
    }
    // Repli cle pour DiscordL : `msg.mentions` n'inclut PAS les mentions situees
    // DANS les embeds (seulement celles du contenu). On parse donc `<@id>`
    // directement dans la description/titre de l'embed.
    for e in &msg.embeds {
        for s in [e.description.as_deref(), e.title.as_deref()]
            .into_iter()
            .flatten()
        {
            if let Some(id) = first_mention_id(s) {
                return Some(UserId::new(id));
            }
        }
    }
    // Repli ultime : toute mention non-bot du message.
    msg.mentions.iter().find(|m| !m.bot).map(|m| m.id)
}

/// Extrait l'ID de la premiere mention utilisateur `<@id>` / `<@!id>` d'un texte.
fn first_mention_id(s: &str) -> Option<u64> {
    let start = s.find("<@")?;
    let rest = s[start + 2..].trim_start_matches('!');
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

#[derive(serde::Serialize)]
struct RecordBumpBody {
    username: String,
    channel_id: String,
    provider: String,
}

#[derive(serde::Deserialize, Default)]
struct BumpRewardResp {
    #[serde(default)]
    rewarded: bool,
    #[serde(default)]
    reward: i64,
    #[serde(default)]
    weekly_count: i64,
    /// Role VIP a poser (None si feature off ou seuil pas atteint).
    #[serde(default)]
    vip_role_id: Option<String>,
    /// True uniquement au bump qui debloque le VIP (annonce one-shot).
    #[serde(default)]
    vip_just_unlocked: bool,
}

/// Appele a chaque EDITION de message. Certains bots de bump (DiscordL)
/// repondent d'abord un message VIDE a l'interaction `/bump`, puis l'editent
/// pour y ajouter l'embed de resultat. On ne voit donc rien au MESSAGE_CREATE :
/// il faut re-lire le message a l'edition, quand l'embed est enfin present.
pub async fn on_message_update(
    ctx: &Context,
    event: &serenity::model::event::MessageUpdateEvent,
) {
    // Un embed vient-il d'apparaitre ? (sinon rien a detecter).
    let has_embeds = event.embeds.as_ref().map(|e| !e.is_empty()).unwrap_or(false);
    if !has_embeds {
        return;
    }
    // Si l'auteur est connu et n'est pas un provider bump, on ignore
    // (evite de refetch tous les messages edites du serveur).
    if let Some(author) = &event.author {
        if provider_for_bot(author.id.get()).is_none() {
            return;
        }
    }
    // Re-lit le message complet (embed + interaction_metadata desormais la).
    if let Ok(msg) = event.channel_id.message(&ctx.http, event.id).await {
        on_message(ctx, &msg).await;
    }
}

/// Appele pour chaque message : si c'est une confirmation de bump reussie d'un
/// provider connu, recompense l'auteur du /bump.
pub async fn on_message(ctx: &Context, msg: &Message) {
    let Some(provider) = provider_for_bot(msg.author.id.get()) else {
        return;
    };
    let Some(guild_id) = msg.guild_id else { return };
    let guild_id = guild_id.to_string();

    let detected_success = (provider.detect)(msg);
    let bumper_id = resolve_bumper(msg);

    // DIAGNOSTIC : trace l'etat du message pour calibrer la detection (la
    // detection DiscordL n'a pas encore ete verifiee en conditions reelles).
    info!(
        provider = provider.key,
        guild_id,
        has_interaction_metadata = msg.interaction_metadata.is_some(),
        is_command_interaction = matches!(
            msg.interaction_metadata.as_deref(),
            Some(MessageInteractionMetadata::Command(_))
        ),
        resolved_bumper = bumper_id.map(|u| u.get()),
        embed_count = msg.embeds.len(),
        embed_title = msg
            .embeds
            .first()
            .and_then(|e| e.title.as_deref())
            .unwrap_or("<aucun>"),
        embed_desc = msg
            .embeds
            .first()
            .and_then(|e| e.description.as_deref())
            .unwrap_or("<aucune>"),
        content = %msg.content,
        detected_success,
        "DIAG bump: message provider recu"
    );

    let Some(bumper_id) = bumper_id else { return };
    if !detected_success {
        return;
    }

    // Module actif pour cette guild ?
    let cfg =
        crate::shared::discord_helpers::guild_config_or_default(ctx, &guild_id, MODULE_BOT_NAME)
            .await;
    if !BaseApiClient::config_bool(&cfg, "enabled", false) {
        return;
    }

    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    // Nom d'affichage du bumpeur (best-effort).
    let username = msg
        .mentions
        .iter()
        .find(|m| m.id == bumper_id)
        .map(|m| m.name.clone())
        .or_else(|| match msg.interaction_metadata.as_deref() {
            Some(MessageInteractionMetadata::Command(meta)) => Some(meta.user.name.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let body = RecordBumpBody {
        username,
        channel_id: msg.channel_id.to_string(),
        provider: provider.key.to_string(),
    };
    let resp: BumpRewardResp = match api
        .post_json(&format!("/api/bump/{}/{}", guild_id, bumper_id), &body)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, guild_id, provider = provider.key, "Echec enregistrement bump");
            return;
        }
    };
    // Role VIP : pose le role (idempotent) des que l'API le renvoie, et
    // annonce le passage VIP une seule fois (vip_just_unlocked). Independant
    // du montant de coins : un bump qui rapporte 0 compte quand meme.
    if let Some(role_id_str) = resp.vip_role_id.as_deref() {
        if let (Ok(gid), Ok(rid)) = (guild_id.parse::<u64>(), role_id_str.parse::<u64>()) {
            let gid = serenity::model::id::GuildId::new(gid);
            let rid = serenity::model::id::RoleId::new(rid);
            if let Err(e) = ctx
                .http
                .add_member_role(
                    gid,
                    bumper_id,
                    rid,
                    Some("Bump VIP — seuil de bumps atteint"),
                )
                .await
            {
                warn!(error = %e, user = %bumper_id, role = %rid, "Echec attribution role VIP bump");
            } else if resp.vip_just_unlocked {
                let vip_msg = format!(
                    "👑 <@{}> est maintenant **VIP** grâce à ses bumps ! Merci pour ton soutien au serveur 🙌",
                    bumper_id
                );
                if let Err(e) = msg.channel_id.say(&ctx.http, vip_msg).await {
                    warn!(error = %e, "Echec annonce passage VIP bump");
                }
                info!(guild_id, user = %bumper_id, "Membre passe VIP via bumps");
            }
        }
    }

    if !resp.rewarded || resp.reward <= 0 {
        return;
    }

    let content = format!(
        "🎉 Merci <@{}> pour le **bump** ({}) ! **+{} coins** (bump #{} de la semaine)",
        bumper_id, provider.display, resp.reward, resp.weekly_count
    );
    if let Err(e) = msg.channel_id.say(&ctx.http, content).await {
        warn!(error = %e, "Echec annonce recompense bump");
    }
    info!(guild_id, user = %bumper_id, provider = provider.key, reward = resp.reward, "Bump recompense");
}

#[derive(serde::Deserialize)]
struct DueReminder {
    guild_id: String,
    channel_id: String,
    #[serde(default = "default_provider")]
    provider: String,
}

fn default_provider() -> String {
    "disboard".to_string()
}

/// Tache de fond : poste un rappel quand le cooldown de bump est ecoule.
pub fn spawn_background(ctx: Context) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let api = {
                let data = ctx.data.read().await;
                data.get::<ApiClientKey>().cloned()
            };
            let Some(api) = api else { continue };
            let due: Vec<DueReminder> = api
                .get_json("/api/bump/due-reminders")
                .await
                .unwrap_or_default();
            for d in due {
                let provider = provider_by_key(&d.provider).unwrap_or(&DISBOARD);
                if let Ok(cid) = d.channel_id.parse::<u64>() {
                    let text = format!(
                        "⏰ Le serveur peut être **bumpé** à nouveau sur **{}** ! Faites `{}` pour le faire remonter — et gagner des coins.",
                        provider.display, provider.bump_hint
                    );
                    let _ = ChannelId::new(cid).say(&ctx.http, text).await;
                }
                // Best-effort : on marque envoye meme si le post echoue, pour ne
                // pas spammer le rappel a chaque tick.
                let _ = api
                    .post_json::<_, serde_json::Value>(
                        &format!("/api/bump/{}/reminder-sent", d.guild_id),
                        &serde_json::json!({ "provider": d.provider }),
                    )
                    .await;
            }
        }
    });
}
