use std::sync::Arc;

use serenity::builder::{
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditChannel,
};
use serenity::model::guild::Member;
use serenity::model::id::{ChannelId, GuildId, RoleId};
use serenity::model::user::User;
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::discord_helpers::is_module_enabled;
use crate::shared::heartbeat::ApiClientKey;

use super::api_client::WelcomeApiClient;
use super::template;

pub const RULES_ACCEPT_ID: &str = "sentinel_rules_accept";

/// Appele quand un nouveau membre rejoint.
/// Compte les HUMAINS (hors bots) via le cache de la guild ; repli sur le
/// compte approximatif Discord (qui inclut les bots) si le cache est vide.
async fn human_member_count(ctx: &Context, guild_id: GuildId) -> u64 {
    // On se base sur le TOTAL fiable de Discord (`member_count`, maintenu par
    // serenity sur add/remove) auquel on retranche les bots vus dans le cache.
    // Compter directement les humains du cache donnait des resultats faux ("1")
    // quand le cache n'est pas entierement peuple. Les bots, eux, sont quasi
    // toujours en cache -> total - bots ≈ humains exacts.
    if let Some(g) = ctx.cache.guild(guild_id) {
        let total = g.member_count;
        let bots = g.members.values().filter(|m| m.user.bot).count() as u64;
        return total.saturating_sub(bots);
    }
    // Pas de cache : repli sur le compte approximatif (inclut les bots).
    guild_id
        .to_partial_guild_with_counts(&ctx.http)
        .await
        .map(|g| g.approximate_member_count.unwrap_or(0))
        .unwrap_or(0)
}

/// Renomme le salon compteur avec le nombre de membres. Independant des
/// messages welcome/leave : ne depend que de `counter_enabled`.
async fn update_counter(
    ctx: &Context,
    counter_enabled: bool,
    counter_channel_id: Option<&String>,
    counter_format: &str,
    member_count: u64,
) {
    if !counter_enabled {
        return;
    }
    let Some(ch_id) = counter_channel_id else { return };
    let Ok(ch) = ch_id.parse::<u64>() else { return };
    let name = counter_format.replace("{count}", &member_count.to_string());
    if let Err(e) = ChannelId::new(ch).edit(&ctx.http, EditChannel::new().name(&name)).await {
        warn!(error = %e, "Echec mise a jour compteur membres");
    }
}

/// Compte les HUMAINS (hors bots) actuellement connectes en vocal sur la
/// guild, via le cache des `voice_states`. Les bots musique/soundboard ne
/// sont pas comptes.
fn voice_member_count(ctx: &Context, guild_id: GuildId) -> u64 {
    let Some(g) = ctx.cache.guild(guild_id) else { return 0 };
    g.voice_states
        .values()
        .filter(|vs| vs.channel_id.is_some())
        .filter(|vs| {
            // Exclut les bots : si le membre est en cache et marque bot, on
            // l'ignore ; sinon on le compte (humain par defaut).
            !g.members.get(&vs.user_id).map(|m| m.user.bot).unwrap_or(false)
        })
        .count() as u64
}

/// Renomme le salon compteur vocal avec le nombre de connectes en vocal.
/// Independant du compteur de membres : ne depend que de
/// `voice_counter_enabled`.
async fn update_voice_counter(
    ctx: &Context,
    enabled: bool,
    channel_id: Option<&String>,
    format: &str,
    voice_count: u64,
) {
    if !enabled {
        return;
    }
    let Some(ch_id) = channel_id else { return };
    let Ok(ch) = ch_id.parse::<u64>() else { return };
    let name = format.replace("{count}", &voice_count.to_string());
    if let Err(e) = ChannelId::new(ch).edit(&ctx.http, EditChannel::new().name(&name)).await {
        warn!(error = %e, "Echec mise a jour compteur vocal");
    }
}

/// Appele a chaque changement d'etat vocal (join/leave/move). Met a jour le
/// salon compteur "En Vocal : N" si la fonctionnalite est activee.
pub async fn on_voice_state_update(
    ctx: &Context,
    old: &Option<serenity::model::voice::VoiceState>,
    new: &serenity::model::voice::VoiceState,
) {
    // guild_id provient de `new`, ou de `old` lors d'une deconnexion totale.
    let guild_id = match new.guild_id.or_else(|| old.as_ref().and_then(|o| o.guild_id)) {
        Some(g) => g,
        None => return,
    };

    if !is_module_enabled(ctx, &guild_id.to_string(), crate::modules::welcome::MODULE_BOT_NAME).await {
        return;
    }

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => Arc::clone(b),
        None => return,
    };
    let grpc = match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
        Some(g) => Arc::clone(g),
        None => return,
    };
    drop(data);

    let api = WelcomeApiClient::new(base, grpc);
    let config = match api.get_config(&guild_id.to_string()).await {
        Ok(c) => c,
        Err(_) => return,
    };

    if !config.voice_counter_enabled {
        return;
    }

    let voice_count = voice_member_count(ctx, guild_id);
    update_voice_counter(
        ctx,
        config.voice_counter_enabled,
        config.voice_counter_channel_id.as_ref(),
        &config.voice_counter_format,
        voice_count,
    )
    .await;
}

pub async fn on_member_add(ctx: &Context, new_member: &Member) {
        let ctx = ctx.clone();
        let new_member = new_member.clone();
        let guild_id = new_member.guild_id;
        let user_id = new_member.user.id;

        // Master switch : si le module est desactive, on saute tout
        // (welcome embed, DM, counter, etc.). Default true.
        if !is_module_enabled(&ctx, &guild_id.to_string(), crate::modules::welcome::MODULE_BOT_NAME).await {
            return;
        }

        let data = ctx.data.read().await;
        let base = match data.get::<ApiClientKey>() {
            Some(b) => Arc::clone(b),
            None => return,
        };
        let grpc = match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
            Some(g) => Arc::clone(g),
            None => return,
        };
        drop(data);

        let api = WelcomeApiClient::new(base.clone(), grpc);
        let config = match api.get_config(&guild_id.to_string()).await {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Echec chargement config welcome");
                return;
            }
        };

        let guild_name = guild_id
            .to_partial_guild(&ctx.http)
            .await
            .map(|g| g.name.clone())
            .unwrap_or_else(|_| "Serveur".into());

        let member_count = human_member_count(&ctx, guild_id).await;

        // ── Detecter si c'est un retour (membre deja connu) ──
        let is_rejoin = api.is_known_member(&guild_id.to_string(), &user_id.to_string()).await;

        // ── Message de bienvenue ──
        if config.welcome_enabled {
            if let Some(ch_id) = &config.welcome_channel_id {
                if let Ok(ch) = ch_id.parse::<u64>() {
                    let channel = ChannelId::new(ch);

                    // Choisir le message : retour ou premiere fois
                    let msg_template = if is_rejoin {
                        &config.rejoin_message
                    } else {
                        &config.welcome_message
                    };

                    let text = template::render(
                        msg_template,
                        &user_id.to_string(),
                        &new_member.user.name,
                        &guild_name,
                        member_count,
                        None,
                    );

                    // Choix title/image/footer selon bienvenue vs retour.
                    let (raw_title, raw_image, raw_footer, default_title) = if is_rejoin {
                        (
                            &config.rejoin_title,
                            &config.rejoin_image_url,
                            &config.rejoin_footer_text,
                            "Bon retour !",
                        )
                    } else {
                        (
                            &config.welcome_title,
                            &config.welcome_image_url,
                            &config.welcome_footer_text,
                            "Bienvenue !",
                        )
                    };
                    let title = if raw_title.is_empty() { default_title.to_string() } else { raw_title.clone() };
                    let color = template::parse_color(&config.welcome_embed_color);
                    let footer_raw = if raw_footer.is_empty() {
                        format!("{} membres", member_count)
                    } else {
                        raw_footer.replace("{count}", &member_count.to_string())
                    };
                    let mut embed = CreateEmbed::new()
                        .title(&title)
                        .description(&text)
                        .color(color)
                        .thumbnail(new_member.user.face())
                        .footer(CreateEmbedFooter::new(footer_raw));
                    if !raw_image.is_empty() {
                        info!(url = %raw_image, is_rejoin, "Ajout image banniere a l embed welcome");
                        embed = embed.image(raw_image.as_str());
                    } else {
                        info!(is_rejoin, "Pas d image banniere configuree");
                    }

                    if let Err(e) = channel.send_message(&ctx.http, CreateMessage::new().embed(embed)).await {
                        warn!(error = %e, "Echec envoi message bienvenue");
                    } else {
                        info!(
                            user = %new_member.user.name,
                            guild = %guild_name,
                            rejoin = is_rejoin,
                            "Message de {} envoye",
                            if is_rejoin { "retour" } else { "bienvenue" }
                        );
                    }
                }
            }
        }

        // ── DM de bienvenue ──
        if config.welcome_dm_enabled {
            let dm_text = template::render(
                &config.welcome_dm_message,
                &user_id.to_string(),
                &new_member.user.name,
                &guild_name,
                member_count,
                None,
            );

            if let Ok(dm_channel) = new_member.user.create_dm_channel(&ctx.http).await {
                if let Err(e) = dm_channel.send_message(&ctx.http, CreateMessage::new().content(&dm_text)).await {
                    warn!(error = %e, user = %new_member.user.name, "Echec envoi DM bienvenue");
                }
            }
        }

        // ── Compteur de membres ──
        update_counter(
            &ctx,
            config.counter_enabled,
            config.counter_channel_id.as_ref(),
            &config.counter_format,
            member_count,
        )
        .await;

        // ── Log ──
        base.send_log("info", &guild_id.to_string(), &format!(
            "Nouveau membre : {} ({})", new_member.user.name, user_id
        ));
    }

/// Appele quand un membre quitte.
pub async fn on_member_remove(ctx: &Context, guild_id: GuildId, user: &User) {
        let ctx = ctx.clone();
        let user = user.clone();

        // Master switch : si le module est desactive, on saute le message de depart.
        if !is_module_enabled(&ctx, &guild_id.to_string(), crate::modules::welcome::MODULE_BOT_NAME).await {
            return;
        }

        let data = ctx.data.read().await;
        let base = match data.get::<ApiClientKey>() {
            Some(b) => Arc::clone(b),
            None => return,
        };
        let grpc = match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
            Some(g) => Arc::clone(g),
            None => return,
        };
        drop(data);

        let api = WelcomeApiClient::new(base.clone(), grpc);
        let config = match api.get_config(&guild_id.to_string()).await {
            Ok(c) => c,
            Err(_) => return,
        };

        // Compteur : INDEPENDANT du message de depart. On le met a jour AVANT
        // les early-returns ci-dessous (sinon un message de depart desactive
        // empechait la mise a jour du compteur au depart d'un membre).
        let member_count = human_member_count(&ctx, guild_id).await;
        update_counter(
            &ctx,
            config.counter_enabled,
            config.counter_channel_id.as_ref(),
            &config.counter_format,
            member_count,
        )
        .await;

        if !config.leave_enabled {
            return;
        }

        let ch_id = match &config.leave_channel_id {
            Some(c) => c,
            None => return,
        };

        let ch = match ch_id.parse::<u64>() {
            Ok(c) => ChannelId::new(c),
            Err(_) => return,
        };

        let guild_name = guild_id
            .to_partial_guild(&ctx.http)
            .await
            .map(|g| g.name.clone())
            .unwrap_or_else(|_| "Serveur".into());

        let text = template::render(
            &config.leave_message,
            &user.id.to_string(),
            &user.name,
            &guild_name,
            member_count,
            None,
        );

        let leave_title = if config.leave_title.is_empty() {
            "Au revoir...".to_string()
        } else {
            config.leave_title.clone()
        };
        let leave_footer = if config.leave_footer_text.is_empty() {
            format!("{} membres", member_count)
        } else {
            config.leave_footer_text.replace("{count}", &member_count.to_string())
        };
        let mut embed = CreateEmbed::new()
            .title(&leave_title)
            .description(&text)
            .color(0xe74c3c)
            .footer(CreateEmbedFooter::new(leave_footer));
        if !config.leave_image_url.is_empty() {
            embed = embed.image(&config.leave_image_url);
        }

        if let Err(e) = ch.send_message(&ctx.http, CreateMessage::new().embed(embed)).await {
            warn!(error = %e, "Echec envoi message depart");
        }
        // (Le compteur a deja ete mis a jour plus haut, avant le return.)
    }

/// Appele pour les interactions de composants (bouton reglement).
pub async fn on_component(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
    if let Some(guild_id) = component.guild_id {
        if !is_module_enabled(ctx, &guild_id.to_string(), crate::modules::welcome::MODULE_BOT_NAME).await {
            return;
        }
    }
    if component.data.custom_id == RULES_ACCEPT_ID {
        handle_rules_accept(ctx, component).await;
    }
}

/// Gere le clic sur le bouton "J'accepte les regles".
/// Poste (ou republie) le panneau de reglement avec le bouton d'acceptation
/// dans le salon configure. Declenche par l'event `welcome_rules_publish`
/// (bouton "Publier le reglement" du dashboard).
pub async fn publish_rules_panel(ctx: &Context, guild_id: GuildId) -> Result<(), String> {
    use serenity::all::ButtonStyle;
    use serenity::builder::{CreateActionRow, CreateButton};

    let (base, grpc) = {
        let data = ctx.data.read().await;
        let base = data
            .get::<ApiClientKey>()
            .map(Arc::clone)
            .ok_or("client API absent")?;
        let grpc = data
            .get::<crate::shared::grpc_client::GrpcClientKey>()
            .map(Arc::clone)
            .ok_or("client gRPC absent")?;
        (base, grpc)
    };

    let api = WelcomeApiClient::new(base, grpc);
    let config = api
        .get_config(&guild_id.to_string())
        .await
        .map_err(|e| format!("lecture config welcome: {e}"))?;

    if !config.rules_enabled {
        return Err("la validation du reglement est desactivee".into());
    }
    let channel_id = config
        .rules_channel_id
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(ChannelId::new)
        .ok_or("aucun salon de reglement configure")?;

    let label = {
        let l = config.rules_button_label.trim();
        if l.is_empty() { "J'accepte les règles".to_string() } else { l.to_string() }
    };

    let embed = CreateEmbed::new()
        .title("📜 Règlement")
        .description(&config.rules_message)
        .color(0x5865f2);
    let button = CreateButton::new(RULES_ACCEPT_ID).label(label).style(ButtonStyle::Success);
    let row = CreateActionRow::Buttons(vec![button]);

    channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed).components(vec![row]))
        .await
        .map_err(|e| format!("envoi du message: {e}"))?;

    info!(guild = %guild_id, channel = %channel_id, "Panneau de reglement publie");
    Ok(())
}

/// Attribue le(s) role(s) configure(s) apres validation du reglement (CSV
/// d'IDs). Retourne le nombre de roles reellement poses, ou `Err` si la
/// fonctionnalite est desactivee / mal configuree (a logger par l'appelant).
/// Partage par le bouton du bot ET le filtrage d'adhesion natif de Discord.
async fn assign_rules_roles(
    ctx: &Context,
    guild_id: GuildId,
    user_id: serenity::model::id::UserId,
) -> Result<usize, String> {
    let (base, grpc) = {
        let data = ctx.data.read().await;
        let base = data.get::<ApiClientKey>().map(Arc::clone).ok_or("api client absent")?;
        let grpc = data
            .get::<crate::shared::grpc_client::GrpcClientKey>()
            .map(Arc::clone)
            .ok_or("grpc client absent")?;
        (base, grpc)
    };

    let api = WelcomeApiClient::new(base, grpc);
    let config = api
        .get_config(&guild_id.to_string())
        .await
        .map_err(|e| format!("lecture config welcome: {e}"))?;

    if !config.rules_enabled {
        return Err("validation du reglement desactivee".into());
    }

    // Liste de roles (CSV d'IDs) : un ancien reglage a role unique reste un
    // CSV a 1 element -> retro-compatible.
    let role_ids: Vec<RoleId> = config
        .rules_role_id
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter_map(|s| s.trim().parse::<u64>().ok())
        .map(RoleId::new)
        .collect();
    if role_ids.is_empty() {
        return Err("aucun role configure".into());
    }

    let mut assigned = 0usize;
    for role_id in &role_ids {
        match ctx
            .http
            .add_member_role(guild_id, user_id, *role_id, Some("Reglement accepte"))
            .await
        {
            Ok(_) => assigned += 1,
            Err(e) => warn!(error = %e, role = %role_id, "Echec assignation role reglement"),
        }
    }
    Ok(assigned)
}

/// Fin du filtrage d'adhesion Discord (membership screening) : `pending`
/// passe de true a false. On attribue alors le(s) role(s) du reglement.
pub async fn on_screening_complete(ctx: &Context, guild_id: GuildId, user_id: serenity::model::id::UserId) {
    match assign_rules_roles(ctx, guild_id, user_id).await {
        Ok(n) if n > 0 => info!(user = %user_id, guild = %guild_id, roles = n, "Roles reglement attribues (filtrage Discord)"),
        Ok(_) => {}
        // Desactive / non configure : silencieux (cas normal sur la plupart
        // des serveurs). Les vraies erreurs d'assignation sont deja loggees.
        Err(_) => {}
    }
}

async fn handle_rules_accept(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };

    let assigned = match assign_rules_roles(ctx, guild_id, component.user.id).await {
        Ok(n) => n,
        Err(_) => return,
    };

    if assigned == 0 {
        let resp = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Erreur lors de l'assignation des roles.")
                .ephemeral(true),
        );
        let _ = component.create_response(&ctx.http, resp).await;
        return;
    }

    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("Reglement accepte ! Bienvenue sur le serveur.")
            .ephemeral(true),
    );
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec reponse acceptation reglement");
    }

    info!(user = %component.user.name, guild = %guild_id, "Reglement accepte");
}

