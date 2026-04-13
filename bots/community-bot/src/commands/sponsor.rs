use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::discord_helpers::reply_ephemeral;
use sentinel_shared::embeds::success_embed;
use sentinel_shared::heartbeat::ApiClientKey;

use crate::handler::{CooldownKey, SponsorshipKey};

pub fn register() -> CreateCommand {
    // Option B : ouvert a tous les membres. Pas de default_member_permissions.
    // Les abus sont prevenus par un ensemble de gardes dans le handler :
    //   - cooldown 30s par user
    //   - anti self-parrain
    //   - anti bot
    //   - target doit etre membre actuel
    //   - parrain doit etre membre du serveur depuis N jours (default 7)
    //   - target doit etre nouveau sur le serveur (default < 30 jours)
    //   - max filleuls actifs par parrain (default 3)
    CreateCommand::new("parrain")
        .description("Parrainer un nouveau membre du serveur")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "membre", "Membre a parrainer")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    // Rate limit anti-spam : 30s par user.
    {
        let data = ctx.data.read().await;
        if let Some(cooldown) = data.get::<CooldownKey>() {
            if let Some(remaining) = cooldown.check_and_set(command.user.id.get(), "parrain", 30) {
                reply_ephemeral(
                    ctx, command,
                    &format!("⏱️ Cooldown actif. Attends {remaining}s avant de parrainer a nouveau."),
                ).await;
                return;
            }
        }
    }

    let target_id = match command.data.options.iter().find(|o| o.name == "membre")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
    {
        Some(id) => id,
        None => { reply_ephemeral(ctx, command, "Parametre membre requis.").await; return; }
    };

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { reply_ephemeral(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    // ══════════════════════════════════════════════════════════════════
    // GARDES ANTI-ABUS
    // ══════════════════════════════════════════════════════════════════

    // 1. Anti self-sponsor (trivialement rapide)
    if target_id == command.user.id {
        reply_ephemeral(ctx, command, "❌ Vous ne pouvez pas vous parrainer vous-meme.").await;
        return;
    }

    // 2. Target != bot
    let target_user = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "❌ Utilisateur introuvable.").await;
            return;
        }
    };
    if target_user.bot {
        reply_ephemeral(ctx, command, "❌ Vous ne pouvez pas parrainer un bot.").await;
        return;
    }

    // 3. Target doit etre membre actuel du serveur
    let target_member = match guild_id.member(&ctx.http, target_id).await {
        Ok(m) => m,
        Err(_) => {
            reply_ephemeral(ctx, command, "❌ Ce membre n'est pas (ou plus) sur le serveur.").await;
            return;
        }
    };

    // 4. Parrain doit etre membre actuel (sanity check)
    let parrain_member = match guild_id.member(&ctx.http, command.user.id).await {
        Ok(m) => m,
        Err(_) => {
            reply_ephemeral(ctx, command, "❌ Erreur : vous devez etre membre du serveur.").await;
            return;
        }
    };

    // Lire la config guild (seuils anti-abus)
    let (max_sponsorships, min_parrain_days, max_filleul_days) = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let gc = match base.get_guild_config(&guild_id.to_string()).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch guild config for sponsorship");
                    std::collections::HashMap::new()
                }
            };
            (
                BaseApiClient::config_u64(&gc, "max_sponsorships", 3) as u32,
                BaseApiClient::config_u64(&gc, "sponsor_min_parrain_days", 7),
                BaseApiClient::config_u64(&gc, "sponsor_max_filleul_days", 30),
            )
        } else {
            (3, 7, 30)
        }
    };

    // 5. Parrain doit etre sur le serveur depuis >= min_parrain_days jours
    let parrain_joined_days = parrain_member.joined_at
        .map(|j| {
            let now = serenity::model::Timestamp::now().unix_timestamp();
            ((now - j.unix_timestamp()) / 86400).max(0) as u64
        })
        .unwrap_or(0);
    if parrain_joined_days < min_parrain_days {
        let remaining = min_parrain_days - parrain_joined_days;
        reply_ephemeral(
            ctx, command,
            &format!(
                "❌ Vous devez etre membre du serveur depuis au moins **{min_parrain_days} jours** pour parrainer. \
                 Encore **{remaining}** jour(s) a attendre."
            ),
        ).await;
        return;
    }

    // 6. Target doit etre un membre recent (< max_filleul_days jours)
    //    Au-dela, il est deja integre et n'a pas besoin d'etre parraine.
    let target_joined_days = target_member.joined_at
        .map(|j| {
            let now = serenity::model::Timestamp::now().unix_timestamp();
            ((now - j.unix_timestamp()) / 86400).max(0) as u64
        })
        .unwrap_or(u64::MAX);
    if target_joined_days > max_filleul_days {
        reply_ephemeral(
            ctx, command,
            &format!(
                "❌ <@{}> est sur le serveur depuis plus de **{max_filleul_days} jours**, il n'est plus eligible au parrainage.",
                target_id
            ),
        ).await;
        return;
    }

    let data = ctx.data.read().await;
    let tracker = match data.get::<SponsorshipKey>() {
        Some(t) => t,
        None => { reply_ephemeral(ctx, command, "Erreur interne.").await; return; }
    };

    // H6 — Check LOCAL d'abord (limite de parrainages). Si le local passe,
    // on appelle l'API AVANT de modifier le tracker definitivement. Si l'API
    // echoue on abort sans avoir perturbe le tracker.
    // Note : tracker.sponsor() modifie le DashMap. Si l'API echoue apres, on
    // doit revert via tracker.remove_sponsor(). L'ordre est :
    //   1. Validation locale (limite)
    //   2. tracker.sponsor() (tentative)
    //   3. API create_sponsorship()
    //   4. Si API KO → tracker.remove_sponsor() (rollback)
    match tracker.sponsor(guild_id.get(), command.user.id.get(), target_id.get(), max_sponsorships) {
        Ok(()) => {
            // Persister via l'API, avec rollback si echec
            let api_result = if let Some(api) = data.get::<crate::handler::RolesApiKey>() {
                api.create_sponsorship(
                    &guild_id.to_string(),
                    &command.user.id.to_string(),
                    &target_id.to_string(),
                ).await
            } else {
                Ok(())
            };

            if let Err(e) = api_result {
                // Rollback du tracker en memoire
                tracker.remove_sponsor(guild_id.get(), command.user.id.get(), target_id.get());
                warn!(error = %e, "Echec API create_sponsorship — rollback tracker local");
                reply_ephemeral(ctx, command, "Echec d'enregistrement du parrainage cote serveur. Rien n'a ete applique.").await;
                return;
            }

            let embed = success_embed("Parrainage enregistre !")
                .description(format!(
                    "<@{}> est maintenant le parrain de <@{}>.\n\
                     Bienvenue dans la communaute !",
                    command.user.id, target_id
                ))
                .field("Parrain", format!("<@{}>", command.user.id), true)
                .field("Filleul", format!("<@{}>", target_id), true);

            if let Err(e) = command.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(embed),
                ),
            ).await {
                warn!(error = %e, "Failed to send sponsorship response");
            }

            // Log + event temps reel
            if let Some(base) = data.get::<ApiClientKey>() {
                base.send_log(
                    "info",
                    &guild_id.to_string(),
                    &format!("{} a parraine {}", command.user.name, target_id),
                );
                base.publish_event("sponsorship_created", serde_json::json!({
                    "guild_id": guild_id.to_string(),
                    "sponsor_id": command.user.id.to_string(),
                    "sponsor_name": command.user.name,
                    "sponsored_id": target_id.to_string(),
                }));
            }

            info!(
                parrain = %command.user.name,
                filleul = %target_id,
                guild = %guild_id,
                "Parrainage enregistre"
            );
        }
        Err(msg) => {
            reply_ephemeral(ctx, command, msg).await;
        }
    }
}

