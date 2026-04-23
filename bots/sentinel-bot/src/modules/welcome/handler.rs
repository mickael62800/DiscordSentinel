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

use sentinel_shared::discord_helpers::is_module_enabled;
use sentinel_shared::heartbeat::ApiClientKey;

use super::api_client::WelcomeApiClient;
use super::template;

pub const RULES_ACCEPT_ID: &str = "sentinel_rules_accept";

/// Appele quand un nouveau membre rejoint.
pub async fn on_member_add(ctx: &Context, new_member: &Member) {
        let ctx = ctx.clone();
        let new_member = new_member.clone();
        let guild_id = new_member.guild_id;
        let user_id = new_member.user.id;

        let data = ctx.data.read().await;
        let base = match data.get::<ApiClientKey>() {
            Some(b) => Arc::clone(b),
            None => return,
        };
        let grpc = match data.get::<sentinel_shared::grpc_client::GrpcClientKey>() {
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

        let member_count = guild_id
            .to_partial_guild_with_counts(&ctx.http)
            .await
            .map(|g| g.approximate_member_count.unwrap_or(0))
            .unwrap_or(0);

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
        if config.counter_enabled {
            if let Some(ch_id) = &config.counter_channel_id {
                if let Ok(ch) = ch_id.parse::<u64>() {
                    let name = config.counter_format.replace("{count}", &member_count.to_string());
                    let edit = EditChannel::new().name(&name);
                    if let Err(e) = ChannelId::new(ch).edit(&ctx.http, edit).await {
                        warn!(error = %e, "Echec mise a jour compteur membres");
                    }
                }
            }
        }

        // ── Log ──
        base.send_log("info", &guild_id.to_string(), &format!(
            "Nouveau membre : {} ({})", new_member.user.name, user_id
        ));
    }

/// Appele quand un membre quitte.
pub async fn on_member_remove(ctx: &Context, guild_id: GuildId, user: &User) {
        let ctx = ctx.clone();
        let user = user.clone();
        let data = ctx.data.read().await;
        let base = match data.get::<ApiClientKey>() {
            Some(b) => Arc::clone(b),
            None => return,
        };
        let grpc = match data.get::<sentinel_shared::grpc_client::GrpcClientKey>() {
            Some(g) => Arc::clone(g),
            None => return,
        };
        drop(data);

        let api = WelcomeApiClient::new(base.clone(), grpc);
        let config = match api.get_config(&guild_id.to_string()).await {
            Ok(c) => c,
            Err(_) => return,
        };

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

        let member_count = guild_id
            .to_partial_guild_with_counts(&ctx.http)
            .await
            .map(|g| g.approximate_member_count.unwrap_or(0))
            .unwrap_or(0);

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

        // Mise a jour compteur
        if config.counter_enabled {
            if let Some(counter_ch) = &config.counter_channel_id {
                if let Ok(c) = counter_ch.parse::<u64>() {
                    let name = config.counter_format.replace("{count}", &member_count.to_string());
                    if let Err(e) = ChannelId::new(c).edit(&ctx.http, EditChannel::new().name(&name)).await {
                        warn!(error = %e, "Echec mise a jour compteur");
                    }
                }
            }
        }
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
async fn handle_rules_accept(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => Arc::clone(b),
        None => return,
    };
    let grpc = match data.get::<sentinel_shared::grpc_client::GrpcClientKey>() {
        Some(g) => Arc::clone(g),
        None => return,
    };
    drop(data);

    let api = WelcomeApiClient::new(base, grpc);
    let config = match api.get_config(&guild_id.to_string()).await {
        Ok(c) => c,
        Err(_) => return,
    };

    if !config.rules_enabled {
        return;
    }

    let role_id = match &config.rules_role_id {
        Some(r) => match r.parse::<u64>() {
            Ok(id) => RoleId::new(id),
            Err(_) => return,
        },
        None => return,
    };

    // Assigner le role
    if let Err(e) = ctx.http.add_member_role(
        guild_id,
        component.user.id,
        role_id,
        Some("Reglement accepte"),
    ).await {
        warn!(error = %e, "Echec assignation role reglement");
        let resp = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Erreur lors de l'assignation du role.")
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

