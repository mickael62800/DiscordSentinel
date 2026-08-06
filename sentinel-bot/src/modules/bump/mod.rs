//! Bump rewards : detecte un /bump reussi (Disboard ou DiscordL), recompense
//! l'auteur en coins (recompense graduee selon le nombre de bumps de la
//! semaine, calculee cote API) et rappelle quand un nouveau bump est possible
//! apres le cooldown.
//!
//! La detection (registre providers, succes/cooldown, resolution du bumpeur)
//! vit dans le core hexagonal (`sentinel_core::domain::services::bump`). Ce
//! module construit le DTO `BumpMessageFacts` depuis le `Message` Serenity et
//! garde l'orchestration Discord/API.

use std::time::Duration;

mod command;

use serenity::all::{CommandInteraction, CreateCommand};

/// Slash commands du module bump (enregistrees si le module est actif).
pub fn register_commands() -> Vec<CreateCommand> {
    vec![command::register()]
}

/// Dispatch de `/bump-statut`.
pub async fn handle_command(ctx: &Context, interaction: &CommandInteraction) {
    command::handle(ctx, interaction).await;
}

use sentinel_core::domain::services::bump::detection::{
    is_provider_bot, provider_by_key, provider_for_message_configured, resolve_bumper, BumpAction,
    BumpMessageFacts, EmbedFacts, UserFacts, DISBOARD, PROVIDERS,
};
use serenity::all::MessageInteractionMetadata;
use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage, EditMessage};
use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, MessageId, UserId};
use serenity::prelude::*;
use tracing::{debug, info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

pub const MODULE_BOT_NAME: &str = "bump-bot";

/// Construit le DTO de detection depuis le message Serenity.
fn message_facts(msg: &Message) -> BumpMessageFacts {
    BumpMessageFacts {
        author_id: msg.author.id.get(),
        embeds: msg
            .embeds
            .iter()
            .map(|e| EmbedFacts {
                title: e.title.clone(),
                description: e.description.clone(),
                fields: e
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), f.value.clone()))
                    .collect(),
            })
            .collect(),
        interaction_user: match msg.interaction_metadata.as_deref() {
            Some(MessageInteractionMetadata::Command(meta)) => Some(UserFacts {
                id: meta.user.id.get(),
                is_bot: meta.user.bot,
            }),
            _ => None,
        },
        mentions: msg
            .mentions
            .iter()
            .map(|m| UserFacts {
                id: m.id.get(),
                is_bot: m.bot,
            })
            .collect(),
    }
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
    // On ne refetch que les editions d'un bot PROVIDER connu (evite de refetch
    // tous les messages edites du serveur, ex: notre propre panneau d'aide).
    if let Some(author) = &event.author {
        if !is_provider_bot(author.id.get()) {
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
    // Seuls les bots (plateformes) postent des confirmations de bump.
    if !msg.author.bot {
        return;
    }
    let Some(guild_id) = msg.guild_id else { return };
    let guild_id = guild_id.to_string();

    // Config lue AVANT le matching : les bot_id des plateformes sont configures
    // par serveur (`{provider}_bot_id`). Master switch d'abord.
    let cfg =
        crate::shared::discord_helpers::guild_config_or_default(ctx, &guild_id, MODULE_BOT_NAME)
            .await;
    if !BaseApiClient::config_bool(&cfg, "enabled", false) {
        return;
    }

    let facts = message_facts(msg);

    // Resolveur : bot_id configure pour une plateforme (0 = non defini -> le
    // core retombe sur le bot_id par defaut, non nul pour Disboard/DiscordL).
    let bot_id_for = |key: &str| -> u64 {
        BaseApiClient::config_or(&cfg, &format!("{key}_bot_id"), "")
            .trim()
            .parse::<u64>()
            .unwrap_or(0)
    };

    let Some(provider) = provider_for_message_configured(&facts, bot_id_for) else {
        // Echec de reconnaissance. On le rend VISIBLE si l'auteur est un bot
        // dont l'ID est configure comme plateforme (ou un provider a bot_id par
        // defaut) : sinon un bump non detecte reste totalement silencieux et
        // impossible a diagnostiquer. La cause la plus frequente : la plateforme
        // confirme en TEXTE SIMPLE (embeds=0), or la detection generique exige
        // un embed.
        let author = msg.author.id.get();
        let est_plateforme_configuree = PROVIDERS.iter().any(|p| {
            let configure = bot_id_for(p.key);
            let effectif = if configure != 0 { configure } else { p.bot_id };
            effectif != 0 && effectif == author
        });
        if est_plateforme_configuree {
            warn!(
                bot_id = author,
                bot_name = %msg.author.name,
                embeds = msg.embeds.len(),
                content = %msg.content.chars().take(200).collect::<String>(),
                embed_desc = msg.embeds.first().and_then(|e| e.description.as_deref()).unwrap_or("<aucune>"),
                "bump: message d'une plateforme configuree NON reconnu (texte simple sans embed ? format change ?)"
            );
        }
        return;
    };

    let detected_success = (provider.detect)(&facts);
    let bumper_id = resolve_bumper(&facts).map(UserId::new);

    debug!(
        provider = provider.key,
        guild_id,
        resolved_bumper = bumper_id.map(|u| u.get()),
        detected_success,
        "bump: message provider traite"
    );

    let Some(bumper_id) = bumper_id else { return };
    if !detected_success {
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
        "🎉 Merci <@{}> pour le **{}** ({}) ! **+{} coins** ({} #{} de la semaine)",
        bumper_id, provider.action.label(), provider.display, resp.reward, provider.action.label(), resp.weekly_count
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

/// Etat d'un provider renvoye par `/api/bump/{guild}/status`.
#[derive(serde::Deserialize)]
struct BumpStatus {
    provider: String,
    ready_at: chrono::DateTime<chrono::Utc>,
}

/// Rafraichit la carte de statut des bumps de chaque guild : un message unique
/// dans le salon des bumps qui liste chaque plateforme (dispo maintenant, ou
/// re-dispo `<t:TS:R>` — compte a rebours qui defile en direct cote Discord).
/// La carte est EDITEE en place (memorisee en memoire par guild).
/// Ref `(channel_id, message_id)` de la carte de statut par guild. PARTAGE entre
/// la boucle de fond et la commande `/bump-statut`, pour qu'elles editent LA
/// MEME carte : sinon la commande posterait un doublon que la boucle continue
/// d'ignorer (elle rafraichit l'ancien message).
static STATUS_CARDS: once_cell::sync::Lazy<Mutex<std::collections::HashMap<u64, (u64, u64)>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(std::collections::HashMap::new()));

/// Issue d'un rafraichissement de carte pour une guild — sert a la commande
/// pour repondre precisement pourquoi rien ne s'affiche.
pub(crate) enum CardRefresh {
    Posted,
    Disabled,
    NoChannel,
    NoPlatforms,
}

/// Rafraichit la carte de CHAQUE guild (boucle de fond).
async fn refresh_all_cards(ctx: &Context) {
    for guild_id in ctx.cache.guilds() {
        let _ = refresh_guild_card(ctx, guild_id.get()).await;
    }
}

/// Construit + poste/edite la carte de statut d'UNE guild.
///
/// Un message unique dans le salon des bumps, qui liste chaque plateforme
/// activee (dispo maintenant, ou re-dispo `<t:TS:R>` — compte a rebours qui
/// defile en direct cote Discord). La carte est EDITEE en place (ref memorisee
/// dans `STATUS_CARDS`).
pub(crate) async fn refresh_guild_card(ctx: &Context, guild_id: u64) -> CardRefresh {
    let api = {
        let data = ctx.data.read().await;
        data.get::<ApiClientKey>().cloned()
    };
    let Some(api) = api else { return CardRefresh::Disabled };

    let gid = guild_id.to_string();
    let cfg =
        crate::shared::discord_helpers::guild_config_or_default(ctx, &gid, MODULE_BOT_NAME).await;
    if !BaseApiClient::config_bool(&cfg, "enabled", false) {
        return CardRefresh::Disabled;
    }
    let Ok(channel_id) = BaseApiClient::config_or(&cfg, "bump_channel_id", "")
        .trim()
        .parse::<u64>()
    else {
        return CardRefresh::NoChannel;
    };

    // Etats connus (cooldown en cours), indexes par provider. Une plateforme
    // jamais bumpee n'a PAS de ligne ici -> elle est simplement "dispo".
    let statuses: Vec<BumpStatus> = api
        .get_json(&format!("/api/bump/{gid}/status"))
        .await
        .unwrap_or_default();
    let ready_by_provider: std::collections::HashMap<&str, chrono::DateTime<chrono::Utc>> =
        statuses
            .iter()
            .map(|s| (s.provider.as_str(), s.ready_at))
            .collect();

    // La carte liste TOUTES les plateformes ACTIVEES (config `{key}_enabled`,
    // defaut vrai comme cote API), qu'elles aient deja ete bumpees ou non.
    // Sans ca, seule la premiere plateforme bumpee (Disboard) apparaissait.
    let now = chrono::Utc::now();
    let mut lines: Vec<String> = Vec::new();
    for p in PROVIDERS {
        if !BaseApiClient::config_bool(&cfg, &format!("{}_enabled", p.key), true) {
            continue;
        }
        let ready_at = ready_by_provider.get(p.key).copied();
        let line = match ready_at {
            Some(t) if t > now => {
                let verb = if p.action == BumpAction::Vote {
                    "vote"
                } else {
                    "bump"
                };
                format!(
                    "⏳ **{}** — {} possible <t:{}:R>",
                    p.display,
                    verb,
                    t.timestamp()
                )
            }
            // Aucun etat, ou cooldown deja ecoule -> disponible maintenant.
            _ => format!(
                "✅ **{}** — disponible **maintenant** ! `{}`",
                p.display, p.bump_hint
            ),
        };
        lines.push(line);
    }
    if lines.is_empty() {
        return CardRefresh::NoPlatforms; // aucune plateforme activee
    }

    let embed = CreateEmbed::new()
        .title("🚀 État des bumps & votes")
        .description(lines.join("\n"))
        .color(0x5865F2)
        .footer(CreateEmbedFooter::new(
            "Mise à jour automatique · les ⏳ défilent en direct",
        ));

    let ch = ChannelId::new(channel_id);
    let mut cards = STATUS_CARDS.lock().await;
    // Edite la carte existante (meme salon), sinon en poste une nouvelle.
    let edited = match cards.get(&guild_id).copied() {
        Some((c, m)) if c == channel_id => ch
            .edit_message(&ctx.http, MessageId::new(m), EditMessage::new().embed(embed.clone()))
            .await
            .is_ok(),
        _ => false,
    };
    if !edited {
        if let Ok(msg) = ch.send_message(&ctx.http, CreateMessage::new().embed(embed)).await {
            cards.insert(guild_id, (channel_id, msg.id.get()));
        }
    }
    CardRefresh::Posted
}

/// Tache de fond : poste un rappel quand le cooldown de bump est ecoule.
///
/// Idempotent : `ready()` refire a chaque reconnexion Discord, mais on ne veut
/// qu'UNE boucle de rappel par process (sinon chaque boucle poste le meme
/// rappel -> doublons/triplons).
pub fn spawn_background(ctx: Context) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    // Carte de statut : editee toutes les 5 min (les <t:R> defilent en direct
    // entre deux rafraichissements). Ref du message gardee en memoire par guild.
    {
        let ctx = ctx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(45)).await;
            loop {
                refresh_all_cards(&ctx).await;
                tokio::time::sleep(Duration::from_secs(300)).await;
            }
        });
    }

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
                    let base_text = if provider.action == BumpAction::Vote {
                        format!(
                            "⏰ Tu peux **voter** à nouveau pour le serveur sur **{}** ! Faites `{}` — et gagnez des coins.",
                            provider.display, provider.bump_hint
                        )
                    } else {
                        format!(
                            "⏰ Le serveur peut être **bumpé** à nouveau sur **{}** ! Faites `{}` pour le faire remonter — et gagner des coins.",
                            provider.display, provider.bump_hint
                        )
                    };

                    // Role a pinger (optionnel), configurable sur la page web.
                    let cfg = crate::shared::discord_helpers::guild_config_or_default(
                        &ctx,
                        &d.guild_id,
                        MODULE_BOT_NAME,
                    )
                    .await;
                    let role_id = BaseApiClient::config_or(&cfg, "bump_reminder_role_id", "")
                        .trim()
                        .parse::<u64>()
                        .ok();

                    let mut msg = serenity::builder::CreateMessage::new();
                    if let Some(rid) = role_id {
                        msg = msg
                            .content(format!("<@&{rid}> {base_text}"))
                            .allowed_mentions(
                                serenity::builder::CreateAllowedMentions::new()
                                    .roles(vec![serenity::model::id::RoleId::new(rid)]),
                            );
                    } else {
                        msg = msg.content(base_text);
                    }
                    let _ = ChannelId::new(cid).send_message(&ctx.http, msg).await;
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
