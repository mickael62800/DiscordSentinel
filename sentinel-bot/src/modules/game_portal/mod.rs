//! Module Game Portal (bot) — "evenements de serveur".
//!
//! Piloté par les events Redis emis par l'API au lancement/arret d'un serveur
//! de jeu. Au demarrage : cree un salon texte + un salon vocal PRIVES (visibles
//! par le role du jeu) dans la categorie configuree, ping le role, et poste un
//! panneau epingle avec bouton d'inscription. A l'arret/suppression : supprime
//! les salons.

use std::sync::Arc;

use serenity::all::{
    ButtonStyle, ChannelId, ChannelType, ComponentInteraction, Context, CreateActionRow,
    CreateButton, CreateChannel, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, GuildId, PermissionOverwrite,
    PermissionOverwriteType, Permissions, RoleId,
};
use serde::Deserialize;

use crate::shared::heartbeat::ApiClientKey;

const MODULE_BOT_NAME: &str = "game-portal";
/// custom_id du bouton d'inscription : `gp_register:{server_id}`.
pub const REGISTER_PREFIX: &str = "gp_register:";

// ── Deserialisation des reponses API ──

#[derive(Deserialize)]
struct ServerDetailResp {
    server: ServerResp,
}

#[derive(Deserialize)]
struct ServerResp {
    guild_id: String,
    template_id: String,
    name: String,
    host_port: Option<u16>,
    ip_reveal_at: Option<String>,
    ip_revealed: bool,
    text_channel_id: Option<String>,
    voice_channel_id: Option<String>,
}

#[derive(Deserialize)]
struct TemplateResp {
    slug: String,
    name: String,
}

#[derive(Deserialize)]
struct SettingsResp {
    template_slug: String,
    discord_role_id: Option<String>,
}

#[derive(Deserialize)]
struct RegResp {
    user_id: String,
}

// ── Consumer Redis ──

/// Spawn le consumer durable (Redis stream). Appele une fois au `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "sentinel-bot-game-portal".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move { handle_event(&ctx, &payload_json).await }
            },
        )
        .await;
    });
}

async fn handle_event(ctx: &Context, payload_json: &str) {
    let env: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    let event = env.get("event").and_then(|v| v.as_str());
    let data = env.get("data");
    let server_id = data
        .and_then(|d| d.get("server_id"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let guild_id = data
        .and_then(|d| d.get("guild_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok());

    let (Some(server_id), Some(guild_id)) = (server_id, guild_id) else {
        return;
    };

    match event {
        Some("game_server_started") => on_started(ctx, GuildId::new(guild_id), &server_id).await,
        Some("game_server_stopped") | Some("game_server_deleted") => {
            on_stopped(ctx, &server_id).await
        }
        Some("game_ip_reveal") => on_ip_reveal(ctx, &server_id).await,
        Some("game_daily_ping") => on_daily_ping(ctx, &server_id).await,
        _ => {}
    }
}

async fn api(ctx: &Context) -> Option<Arc<crate::shared::api_client::BaseApiClient>> {
    ctx.data.read().await.get::<ApiClientKey>().cloned()
}

// ── Lancement d'un serveur -> creation des salons ──

async fn on_started(ctx: &Context, guild_id: GuildId, server_id: &str) {
    let Some(base) = api(ctx).await else { return };

    // 1. Detail du serveur.
    let detail: ServerDetailResp = match base
        .get_json(&format!("/api/games/servers/{server_id}"))
        .await
    {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(error = %e, server_id, "game-portal: echec lecture serveur");
            return;
        }
    };
    let server = detail.server;
    // Salons deja crees ? (event rejoue) -> ne rien refaire.
    if server.text_channel_id.is_some() {
        return;
    }

    // 2. Template (slug + nom lisible).
    let template: Option<TemplateResp> = base
        .get_json(&format!("/api/games/templates/{}", server.template_id))
        .await
        .ok();
    let template_slug = template.as_ref().map(|t| t.slug.clone());
    let game_name = template
        .as_ref()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "Jeu".into());

    // 3. Role a pinguer (reglage du template sur cette guild).
    let role_id = match template_slug {
        Some(slug) => resolve_role(&base, &server.guild_id, &slug).await,
        None => None,
    };

    // 4. Categorie de session (config game-portal).
    let cfg = base
        .get_guild_config_for(&server.guild_id, MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let category = cfg
        .get("session_category_id")
        .and_then(|s| s.parse::<u64>().ok())
        .map(ChannelId::new);

    // 5. Permissions : prive (seul le role du jeu voit).
    let overwrites = build_overwrites(guild_id, role_id);

    // 6. Creation des salons.
    let text_ch = create_channel(
        ctx,
        guild_id,
        &format!("🎮-{}", slugify(&server.name)),
        ChannelType::Text,
        category,
        &overwrites,
    )
    .await;
    let voice_ch = create_channel(
        ctx,
        guild_id,
        &format!("🔊 {}", server.name),
        ChannelType::Voice,
        category,
        &overwrites,
    )
    .await;

    let Some(text_ch) = text_ch else { return };

    // 7. Enregistre les salons cote API : CLAIM garde anti-doublon (D). Si le
    // claim echoue (claimed=false), des salons etaient deja enregistres (event
    // de demarrage rejoue) -> on supprime ceux qu'on vient de creer et on
    // s'arrete (pas de 2e panneau/ping). Une erreur reseau (Err) laisse les
    // salons en place (comportement d'avant, pas de suppression a tort).
    match base
        .patch_json::<_, serde_json::Value>(
            &format!("/api/games/servers/{server_id}/session-channels"),
            &serde_json::json!({
                "text_channel_id": text_ch.to_string(),
                "voice_channel_id": voice_ch.map(|c| c.to_string()),
            }),
        )
        .await
    {
        Ok(v) => {
            let claimed = v.get("claimed").and_then(|c| c.as_bool()).unwrap_or(true);
            if !claimed {
                let _ = text_ch.delete(&ctx.http).await;
                if let Some(vc) = voice_ch {
                    let _ = vc.delete(&ctx.http).await;
                }
                tracing::warn!(server_id, "game-portal: salons deja enregistres (event rejoue) -> doublons supprimes");
                return;
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, server_id, "game-portal: echec enregistrement salons (salons conserves)");
        }
    }

    // 8. Panneau epingle + bouton d'inscription.
    let embed = build_panel_embed(&game_name, &server.name, &[], server.ip_reveal_at.as_deref(), server.ip_revealed, server.host_port);
    let msg = text_ch
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .embed(embed)
                .components(vec![register_row(server_id)]),
        )
        .await;
    if let Ok(m) = &msg {
        let _ = text_ch.pin(&ctx.http, m.id).await;
    }

    // 9. Ping du role.
    if let Some(rid) = role_id {
        let _ = text_ch
            .send_message(
                &ctx.http,
                CreateMessage::new().content(format!(
                    "<@&{}> un serveur **{}** ouvre bientot ! Inscris-toi ci-dessus. 🎮",
                    rid, game_name
                )),
            )
            .await;
    }

    tracing::info!(guild = %guild_id, server_id, "game-portal: session ouverte (salons crees)");
}

async fn resolve_role(
    base: &crate::shared::api_client::BaseApiClient,
    guild_id: &str,
    slug: &str,
) -> Option<RoleId> {
    let settings: Vec<SettingsResp> = base
        .get_json(&format!("/api/games/{guild_id}/template-settings"))
        .await
        .ok()?;
    settings
        .into_iter()
        .find(|s| s.template_slug == slug)
        .and_then(|s| s.discord_role_id)
        .and_then(|r| r.parse::<u64>().ok())
        .map(RoleId::new)
}

/// Permissions : salon prive. @everyone ne voit rien ; le role du jeu voit et
/// peut ecrire / se connecter.
fn build_overwrites(guild_id: GuildId, role_id: Option<RoleId>) -> Vec<PermissionOverwrite> {
    let mut ows = vec![PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::VIEW_CHANNEL,
        // @everyone a le meme id que la guild.
        kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
    }];
    if let Some(rid) = role_id {
        ows.push(PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL
                | Permissions::SEND_MESSAGES
                | Permissions::CONNECT
                | Permissions::SPEAK
                | Permissions::READ_MESSAGE_HISTORY,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Role(rid),
        });
    }
    ows
}

async fn create_channel(
    ctx: &Context,
    guild_id: GuildId,
    name: &str,
    kind: ChannelType,
    category: Option<ChannelId>,
    overwrites: &[PermissionOverwrite],
) -> Option<ChannelId> {
    let mut builder = CreateChannel::new(name).kind(kind).permissions(overwrites.to_vec());
    if let Some(cat) = category {
        builder = builder.category(cat);
    }
    match guild_id.create_channel(&ctx.http, builder).await {
        Ok(ch) => Some(ch.id),
        Err(e) => {
            tracing::warn!(error = %e, name, "game-portal: echec creation salon");
            None
        }
    }
}

// ── Arret d'un serveur -> suppression des salons ──

async fn on_stopped(ctx: &Context, server_id: &str) {
    let Some(base) = api(ctx).await else { return };
    let detail: ServerDetailResp = match base
        .get_json(&format!("/api/games/servers/{server_id}"))
        .await
    {
        Ok(d) => d,
        Err(_) => return,
    };
    for id in [detail.server.text_channel_id, detail.server.voice_channel_id]
        .into_iter()
        .flatten()
        .filter_map(|s| s.parse::<u64>().ok())
    {
        let _ = ChannelId::new(id).delete(&ctx.http).await;
    }
    // Vide les salons cote API.
    base.patch_fire_and_forget(
        &format!("/api/games/servers/{server_id}/session-channels"),
        &serde_json::json!({ "text_channel_id": null, "voice_channel_id": null }),
    )
    .await;
    tracing::info!(server_id, "game-portal: session fermee (salons supprimes)");
}

// ── Revelation d'IP (event worker) ──

async fn on_ip_reveal(ctx: &Context, server_id: &str) {
    let Some(base) = api(ctx).await else { return };
    let Ok(detail) = base
        .get_json::<ServerDetailResp>(&format!("/api/games/servers/{server_id}"))
        .await
    else {
        return;
    };
    let server = detail.server;
    let Some(text_ch) = server
        .text_channel_id
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(ChannelId::new)
    else {
        return;
    };

    let template: Option<TemplateResp> = base
        .get_json(&format!("/api/games/templates/{}", server.template_id))
        .await
        .ok();
    let game_name = template
        .as_ref()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "Jeu".into());
    let role_id = match template.as_ref().map(|t| t.slug.clone()) {
        Some(slug) => resolve_role(&base, &server.guild_id, &slug).await,
        None => None,
    };

    // Adresse : {host public}:{port} si l'hote est configure, sinon le port seul.
    let cfg = base
        .get_guild_config_for(&server.guild_id, MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let host = cfg.get("session_public_host").cloned().unwrap_or_default();
    let addr = match (host.trim().is_empty(), server.host_port) {
        (false, Some(p)) => format!("`{}:{}`", host.trim(), p),
        (true, Some(p)) => format!("port `{p}`"),
        _ => "_communiquee par le staff_".to_string(),
    };

    let ping = role_id
        .map(|r| format!("<@&{r}> "))
        .unwrap_or_default();
    let _ = text_ch
        .send_message(
            &ctx.http,
            CreateMessage::new().content(format!(
                "{ping}🎉 Le serveur **{game_name}** est **OUVERT** ! Connexion : {addr}"
            )),
        )
        .await;

    // Met a jour le panneau epingle (IP desormais visible).
    let regs: Vec<RegResp> = base
        .get_json(&format!("/api/games/servers/{server_id}/registrations"))
        .await
        .unwrap_or_default();
    let user_ids: Vec<String> = regs.into_iter().map(|r| r.user_id).collect();
    let embed = build_panel_embed(&game_name, &server.name, &user_ids, None, true, server.host_port);
    if let Ok(pins) = text_ch.pins(&ctx.http).await {
        if let Some(m) = pins.into_iter().find(|m| !m.embeds.is_empty()) {
            let _ = text_ch
                .edit_message(
                    &ctx.http,
                    m.id,
                    serenity::builder::EditMessage::new()
                        .embed(embed)
                        .components(vec![register_row(server_id)]),
                )
                .await;
        }
    }

    tracing::info!(server_id, "game-portal: IP revelee");
}

// ── Ping quotidien (event worker) ──

async fn on_daily_ping(ctx: &Context, server_id: &str) {
    let Some(base) = api(ctx).await else { return };
    let Ok(detail) = base
        .get_json::<ServerDetailResp>(&format!("/api/games/servers/{server_id}"))
        .await
    else {
        return;
    };
    let server = detail.server;
    let Some(text_ch) = server
        .text_channel_id
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(ChannelId::new)
    else {
        return;
    };

    let template: Option<TemplateResp> = base
        .get_json(&format!("/api/games/templates/{}", server.template_id))
        .await
        .ok();
    let game_name = template
        .as_ref()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "Jeu".into());
    let role_id = match template.as_ref().map(|t| t.slug.clone()) {
        Some(slug) => resolve_role(&base, &server.guild_id, &slug).await,
        None => None,
    };
    let Some(rid) = role_id else { return };

    // Jours restants avant la revelation.
    let remaining = server.ip_reveal_at.as_deref().and_then(|d| {
        chrono::DateTime::parse_from_rfc3339(d).ok().map(|dt| {
            (dt.with_timezone(&chrono::Utc) - chrono::Utc::now())
                .num_days()
                .max(0)
        })
    });
    let when = match remaining {
        Some(0) => "aujourd'hui".to_string(),
        Some(n) => format!("dans **{n}** jour(s)"),
        None => "bientot".to_string(),
    };

    let _ = text_ch
        .send_message(
            &ctx.http,
            CreateMessage::new().content(format!(
                "<@&{rid}> ⏳ Le serveur **{game_name}** ouvre {when} ! Inscris-toi sur le panneau. 🎮"
            )),
        )
        .await;
}

// ── Bouton d'inscription ──

pub fn handles_component(custom_id: &str) -> bool {
    custom_id.starts_with(REGISTER_PREFIX)
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let Some(server_id) = component.data.custom_id.strip_prefix(REGISTER_PREFIX) else {
        return;
    };
    let Some(base) = api(ctx).await else { return };

    let reg_result: Result<serde_json::Value, String> = base
        .post_json(
            &format!("/api/games/servers/{server_id}/registrations"),
            &serde_json::json!({ "user_id": component.user.id.to_string() }),
        )
        .await;
    // L'API peut refuser (serveur ferme, capacite, etc.) : on ne pretend pas que
    // l'inscription a reussi -> message ephemere et on s'arrete.
    if let Err(e) = reg_result {
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!("❌ Inscription impossible : {e}"))
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    // Re-fetch inscrits + serveur pour reconstruire le panneau.
    let regs: Vec<RegResp> = base
        .get_json(&format!("/api/games/servers/{server_id}/registrations"))
        .await
        .unwrap_or_default();
    let user_ids: Vec<String> = regs.into_iter().map(|r| r.user_id).collect();

    if let Ok(detail) = base
        .get_json::<ServerDetailResp>(&format!("/api/games/servers/{server_id}"))
        .await
    {
        let template: Option<TemplateResp> = base
            .get_json(&format!("/api/games/templates/{}", detail.server.template_id))
            .await
            .ok();
        let game_name = template.map(|t| t.name).unwrap_or_else(|| "Jeu".into());
        let embed = build_panel_embed(
            &game_name,
            &detail.server.name,
            &user_ids,
            detail.server.ip_reveal_at.as_deref(),
            detail.server.ip_revealed,
            detail.server.host_port,
        );
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(vec![register_row(server_id)]),
                ),
            )
            .await;
        return;
    }

    // Fallback : simple accuse ephemere.
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("✅ Inscription enregistree.")
                    .ephemeral(true),
            ),
        )
        .await;
}

// ── Panneau ──

fn register_row(server_id: &str) -> CreateActionRow {
    CreateActionRow::Buttons(vec![CreateButton::new(format!("{REGISTER_PREFIX}{server_id}"))
        .label("Je m'inscris")
        .emoji('✅')
        .style(ButtonStyle::Success)])
}

fn build_panel_embed(
    game_name: &str,
    server_name: &str,
    inscrits: &[String],
    ip_reveal_at: Option<&str>,
    ip_revealed: bool,
    host_port: Option<u16>,
) -> CreateEmbed {
    let inscrits_txt = if inscrits.is_empty() {
        "_Personne pour l'instant — sois le premier !_".to_string()
    } else {
        inscrits
            .iter()
            .map(|u| format!("<@{u}>"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let ip_txt = if ip_revealed {
        match host_port {
            Some(p) => format!("**Serveur ouvert !** Port : `{p}`"),
            None => "**Serveur ouvert !**".to_string(),
        }
    } else {
        match ip_reveal_at {
            Some(d) => format!("🔒 Masquee — revelee le **{}**", &d[..10.min(d.len())]),
            None => "🔒 Masquee".to_string(),
        }
    };

    CreateEmbed::new()
        .title(format!("🎮 {game_name} — {server_name}"))
        .description("Un serveur de jeu est en preparation. Inscris-toi pour etre prevenu a l'ouverture !")
        .field(format!("Inscrits ({})", inscrits.len()), inscrits_txt, false)
        .field("Adresse (IP)", ip_txt, false)
        .color(0x5865f2)
        .footer(CreateEmbedFooter::new("Game Portal | Sentinel"))
        .timestamp(serenity::model::Timestamp::now())
}

/// Nettoie un nom pour en faire un nom de salon Discord valide (texte).
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(90)
        .collect()
}
