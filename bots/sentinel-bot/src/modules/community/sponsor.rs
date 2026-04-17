use serenity::all::{
    ButtonStyle, CommandDataOptionValue, CommandInteraction, CommandOptionType,
    ComponentInteraction, Context, CreateActionRow, CreateButton, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::builder::{CreateEmbed, CreateMessage};
use tracing::{info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::discord_helpers::reply_ephemeral;
use sentinel_shared::embeds::success_embed;
use sentinel_shared::heartbeat::ApiClientKey;

use super::{CooldownKey, SponsorshipKey};

pub fn register() -> CreateCommand {
    // Ouvert a tous, mais le filleul doit confirmer avant enregistrement.
    CreateCommand::new("parrain")
        .description("Parrainer un nouveau membre du serveur (validation du filleul requise)")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "membre", "Membre a parrainer")
                .required(true),
        )
}

/// Point d'entree du slash /parrain.
/// Verifie les gardes anti-abus puis envoie une demande de confirmation
/// au filleul — rien n'est enregistre tant qu'il n'a pas clique "Accepter".
pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    // Rate limit anti-spam : 30s par user.
    {
        let data = ctx.data.read().await;
        if let Some(cooldown) = data.get::<CooldownKey>() {
            if let Some(remaining) = cooldown.check_and_set(command.user.id.get(), "parrain", 30) {
                reply_ephemeral(
                    ctx, command,
                    &format!("\u{23f1}\u{fe0f} Cooldown actif. Attends {remaining}s avant de parrainer a nouveau."),
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

    // Valider les gardes. Si OK, on envoie la demande de confirmation.
    match validate_sponsorship(ctx, command.user.id, target_id, guild_id).await {
        Err(msg) => {
            reply_ephemeral(ctx, command, &msg).await;
        }
        Ok(()) => {
            // On envoie un DM au filleul avec les boutons Accepter/Refuser.
            // Le parrain recoit juste une confirmation ephemere, pour ne pas
            // polluer le salon. Si le DM echoue (DMs fermes), on tombe en
            // fallback sur un message public taggant le filleul.
            let embed = CreateEmbed::new()
                .title("\u{1f91d} Demande de parrainage")
                .description(format!(
                    "<@{parrain}> souhaite te parrainer sur le serveur **{guild_name}**.\n\n\
                     **Toi seul(e)** peux accepter ou refuser cette demande.",
                    parrain = command.user.id.get(),
                    guild_name = guild_id
                        .name(&ctx.cache)
                        .unwrap_or_else(|| "ce serveur".to_string()),
                ))
                .color(0x5865F2);

            // Format : sponsor_{accept|refuse}:{guild_id}:{parrain_id}:{target_id}
            // On encode le guild_id car le bouton peut etre clique depuis un DM
            // (ou component.guild_id est None).
            let accept_id = format!(
                "sponsor_accept:{}:{}:{}",
                guild_id.get(),
                command.user.id.get(),
                target_id.get()
            );
            let refuse_id = format!(
                "sponsor_refuse:{}:{}:{}",
                guild_id.get(),
                command.user.id.get(),
                target_id.get()
            );

            let row = CreateActionRow::Buttons(vec![
                CreateButton::new(&accept_id)
                    .label("Accepter")
                    .style(ButtonStyle::Success),
                CreateButton::new(&refuse_id)
                    .label("Refuser")
                    .style(ButtonStyle::Danger),
            ]);

            // Essayer d'envoyer le DM en premier
            let dm_result = async {
                let dm = target_id.create_dm_channel(&ctx.http).await?;
                dm.send_message(
                    &ctx.http,
                    CreateMessage::new().embed(embed.clone()).components(vec![row.clone()]),
                )
                .await
            }
            .await;

            match dm_result {
                Ok(_) => {
                    // DM envoye : reponse ephemere au parrain
                    let ack = CreateEmbed::new()
                        .title("\u{2709}\u{fe0f} Demande envoyee")
                        .description(format!(
                            "Ta demande de parrainage a ete envoyee a <@{}> par message prive.\n\
                             Tu recevras une confirmation dans ce salon si le filleul accepte.",
                            target_id.get()
                        ))
                        .color(0x5865F2);
                    let msg = CreateInteractionResponseMessage::new()
                        .embed(ack)
                        .ephemeral(true);
                    if let Err(e) = command
                        .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
                        .await
                    {
                        warn!(error = %e, "Failed to ack sponsorship DM");
                    }
                }
                Err(e) => {
                    // Fallback : DM ferme. Envoyer un message public dans le
                    // salon d'ou la commande a ete lancee, en taggant le filleul.
                    warn!(error = %e, "DM ferme, fallback public sponsorship request");

                    let public_msg = CreateMessage::new()
                        .content(format!("<@{}>", target_id.get()))
                        .embed(embed)
                        .components(vec![row]);

                    if let Err(e2) = command
                        .channel_id
                        .send_message(&ctx.http, public_msg)
                        .await
                    {
                        warn!(error = %e2, "Failed to send public fallback sponsorship request");
                    }

                    let ack = CreateEmbed::new()
                        .title("\u{2709}\u{fe0f} Demande envoyee (mode public)")
                        .description(format!(
                            "Les DM de <@{}> sont fermes. La demande a ete postee dans ce salon.",
                            target_id.get()
                        ))
                        .color(0xFFA500);
                    let ack_msg = CreateInteractionResponseMessage::new()
                        .embed(ack)
                        .ephemeral(true);
                    if let Err(e3) = command
                        .create_response(&ctx.http, CreateInteractionResponse::Message(ack_msg))
                        .await
                    {
                        warn!(error = %e3, "Failed to ack sponsorship fallback");
                    }
                }
            }
        }
    }
}

/// Handler des boutons `sponsor_accept:{parrain_id}:{target_id}` et
/// `sponsor_refuse:{parrain_id}:{target_id}`.
/// Seul le filleul (target) a le droit de cliquer.
pub async fn handle_button(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();
    let (kind, rest) = if let Some(r) = custom_id.strip_prefix("sponsor_accept:") {
        ("accept", r)
    } else if let Some(r) = custom_id.strip_prefix("sponsor_refuse:") {
        ("refuse", r)
    } else {
        return;
    };

    // Parse "guild_id:parrain_id:target_id"
    let mut parts = rest.split(':');
    let guild_id_raw = parts.next().unwrap_or("");
    let parrain_id_raw = parts.next().unwrap_or("");
    let target_id_raw = parts.next().unwrap_or("");
    let guild_id_u64: u64 = match guild_id_raw.parse() {
        Ok(v) => v,
        Err(_) => return,
    };
    let parrain_id: u64 = match parrain_id_raw.parse() {
        Ok(v) => v,
        Err(_) => return,
    };
    let target_id: u64 = match target_id_raw.parse() {
        Ok(v) => v,
        Err(_) => return,
    };

    // SECURITE : seul le filleul peut cliquer.
    if component.user.id.get() != target_id {
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Seul le membre concerne peut accepter ou refuser ce parrainage.")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    // Le clic peut venir d'un DM — on s'appuie sur le guild_id encode.
    let guild_id = serenity::model::id::GuildId::new(guild_id_u64);

    if kind == "refuse" {
        let embed = CreateEmbed::new()
            .title("\u{274c} Parrainage refuse")
            .description(format!(
                "Tu as refuse la demande de parrainage de <@{parrain_id}>."
            ))
            .color(0xED4245);

        let msg = CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(vec![]);
        if let Err(e) = component
            .create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg))
            .await
        {
            warn!(error = %e, "Failed to update sponsorship refuse");
        }

        // Notifier le parrain en DM (best-effort, ignore si ferme)
        let parrain_uid = serenity::model::id::UserId::new(parrain_id);
        if let Ok(dm) = parrain_uid.create_dm_channel(&ctx.http).await {
            let refuse_embed = CreateEmbed::new()
                .title("\u{274c} Parrainage refuse")
                .description(format!(
                    "<@{target_id}> a refuse ta demande de parrainage."
                ))
                .color(0xED4245);
            let _ = dm
                .send_message(&ctx.http, CreateMessage::new().embed(refuse_embed))
                .await;
        }

        info!(parrain = parrain_id, target = target_id, "Parrainage refuse par le filleul");
        return;
    }

    // kind == "accept" : on re-valide les gardes avant d'enregistrer.
    // Entre la demande initiale et le clic d'acceptation, les conditions
    // ont pu changer (limite atteinte, filleul a deja un parrain, etc.).
    let parrain_uid = serenity::model::id::UserId::new(parrain_id);
    let target_uid = serenity::model::id::UserId::new(target_id);

    if let Err(msg) = validate_sponsorship(ctx, parrain_uid, target_uid, guild_id).await {
        let embed = CreateEmbed::new()
            .title("\u{274c} Parrainage impossible")
            .description(msg)
            .color(0xED4245);
        let response = CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(vec![]);
        if let Err(e) = component
            .create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(response))
            .await
        {
            warn!(error = %e, "Failed to update sponsorship invalid");
        }
        return;
    }

    // Enregistrer : tracker local → API → embed final.
    let data = ctx.data.read().await;

    let max_sponsorships: u32 = if let Some(base) = data.get::<ApiClientKey>() {
        let gc = base
            .get_guild_config(&guild_id.to_string())
            .await
            .unwrap_or_default();
        BaseApiClient::config_u64(&gc, "max_sponsorships", 3) as u32
    } else {
        3
    };

    let tracker = match data.get::<SponsorshipKey>() {
        Some(t) => t,
        None => {
            warn!("SponsorshipKey manquant");
            return;
        }
    };

    if let Err(msg) = tracker.sponsor(guild_id.get(), parrain_id, target_id, max_sponsorships) {
        let embed = CreateEmbed::new()
            .title("\u{274c} Parrainage impossible")
            .description(msg)
            .color(0xED4245);
        let response = CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(vec![]);
        if let Err(e) = component
            .create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(response))
            .await
        {
            warn!(error = %e, "Failed to update sponsorship tracker error");
        }
        return;
    }

    // Persister via l'API, rollback tracker si echec.
    let api_result = if let Some(api) = data.get::<super::RolesApiKey>() {
        api.create_sponsorship(
            &guild_id.to_string(),
            &parrain_id.to_string(),
            &target_id.to_string(),
        )
        .await
    } else {
        Ok(())
    };

    if let Err(e) = api_result {
        tracker.remove_sponsor(guild_id.get(), parrain_id, target_id);
        warn!(error = %e, "Echec API create_sponsorship — rollback tracker local");
        let embed = CreateEmbed::new()
            .title("\u{274c} Erreur serveur")
            .description("Echec d'enregistrement du parrainage. Rien n'a ete applique.")
            .color(0xED4245);
        let response = CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(vec![]);
        let _ = component
            .create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(response))
            .await;
        return;
    }

    // Succes — confirmation au filleul (mise a jour du message DM)
    let embed = success_embed("Parrainage confirme !")
        .description(format!(
            "Tu as accepte le parrainage de <@{parrain_id}>.\n\
             Bienvenue dans la communaute !"
        ))
        .field("Parrain", format!("<@{parrain_id}>"), true)
        .field("Filleul", format!("<@{target_id}>"), true);

    let response = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(vec![]);
    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(response))
        .await
    {
        warn!(error = %e, "Failed to send sponsorship confirmation");
    }

    // Notifier le parrain en DM
    let parrain_uid = serenity::model::id::UserId::new(parrain_id);
    if let Ok(dm) = parrain_uid.create_dm_channel(&ctx.http).await {
        let notif = success_embed("\u{2705} Parrainage accepte")
            .description(format!(
                "<@{target_id}> a accepte ta demande de parrainage. \
                 Tu es maintenant officiellement son parrain !"
            ));
        let _ = dm
            .send_message(&ctx.http, CreateMessage::new().embed(notif))
            .await;
    }

    // Log + event temps reel
    if let Some(base) = data.get::<ApiClientKey>() {
        base.send_log(
            "info",
            &guild_id.to_string(),
            &format!("{parrain_id} a parraine {target_id} (accepte par le filleul)"),
        );
        base.publish_event(
            "sponsorship_created",
            serde_json::json!({
                "guild_id": guild_id.to_string(),
                "sponsor_id": parrain_id.to_string(),
                "sponsored_id": target_id.to_string(),
            }),
        );
    }

    info!(
        parrain = parrain_id,
        filleul = target_id,
        guild = %guild_id,
        "Parrainage confirme par le filleul"
    );
}

/// Verifie tous les gardes anti-abus. Retourne Err(msg) si un garde echoue.
/// Utilise a la fois au lancement du /parrain et au clic d'acceptation du
/// filleul (re-validation : les conditions ont pu changer entre temps).
async fn validate_sponsorship(
    ctx: &Context,
    parrain_id: serenity::model::id::UserId,
    target_id: serenity::model::id::UserId,
    guild_id: serenity::model::id::GuildId,
) -> Result<(), String> {
    // 1. Anti self-sponsor
    if target_id == parrain_id {
        return Err("\u{274c} Vous ne pouvez pas vous parrainer vous-meme.".to_string());
    }

    // 2. Target != bot
    let target_user = target_id
        .to_user(&ctx.http)
        .await
        .map_err(|_| "\u{274c} Utilisateur introuvable.".to_string())?;
    if target_user.bot {
        return Err("\u{274c} Vous ne pouvez pas parrainer un bot.".to_string());
    }

    // 3. Target doit etre membre actuel du serveur
    let target_member = guild_id
        .member(&ctx.http, target_id)
        .await
        .map_err(|_| "\u{274c} Ce membre n'est pas (ou plus) sur le serveur.".to_string())?;

    // 4. Parrain doit etre membre actuel
    let parrain_member = guild_id
        .member(&ctx.http, parrain_id)
        .await
        .map_err(|_| "\u{274c} Le parrain n'est plus sur le serveur.".to_string())?;

    // Lire la config guild
    let (min_parrain_days, max_filleul_days) = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let gc = base
                .get_guild_config(&guild_id.to_string())
                .await
                .unwrap_or_default();
            (
                BaseApiClient::config_u64(&gc, "sponsor_min_parrain_days", 7),
                BaseApiClient::config_u64(&gc, "sponsor_max_filleul_days", 30),
            )
        } else {
            (7, 30)
        }
    };

    // 5. Parrain doit etre sur le serveur depuis >= min_parrain_days jours
    let parrain_joined_days = parrain_member
        .joined_at
        .map(|j| {
            let now = serenity::model::Timestamp::now().unix_timestamp();
            ((now - j.unix_timestamp()) / 86400).max(0) as u64
        })
        .unwrap_or(0);
    if parrain_joined_days < min_parrain_days {
        let remaining = min_parrain_days - parrain_joined_days;
        return Err(format!(
            "\u{274c} Le parrain doit etre membre depuis au moins **{min_parrain_days} jours**. \
             Encore **{remaining}** jour(s) a attendre."
        ));
    }

    // 6. Target doit etre un membre recent (< max_filleul_days jours)
    let target_joined_days = target_member
        .joined_at
        .map(|j| {
            let now = serenity::model::Timestamp::now().unix_timestamp();
            ((now - j.unix_timestamp()) / 86400).max(0) as u64
        })
        .unwrap_or(u64::MAX);
    if target_joined_days > max_filleul_days {
        return Err(format!(
            "\u{274c} <@{}> est sur le serveur depuis plus de **{max_filleul_days} jours**, \
             il n'est plus eligible au parrainage.",
            target_id.get()
        ));
    }

    // 7. Le filleul n'a pas deja un parrain (verification in-memory)
    {
        let data = ctx.data.read().await;
        if let Some(tracker) = data.get::<SponsorshipKey>() {
            if tracker.is_sponsored(guild_id.get(), target_id.get()) {
                return Err("\u{274c} Ce membre a deja un parrain.".to_string());
            }
        }
    }

    Ok(())
}
