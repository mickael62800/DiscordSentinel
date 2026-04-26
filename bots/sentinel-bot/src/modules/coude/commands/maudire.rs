//! Commande `/maudire @cible [type]` (cf. COUPE_AMELIORATIONS 5.1).
//!
//! Pose une malediction ridicule sur un autre joueur pendant 24h.
//! Cout : 300c. Tirage aleatoire si type non specifie. Une seule
//! malediction active par cible — re-cast bloque cote API.

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use sentinel_shared::discord_helpers::reply_ephemeral;

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("maudire")
        .description("Pose une malediction ridicule sur un pote pendant 24h (300c)")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Le pote a maudire")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "type",
                "Type de malediction (aleatoire si non choisi)",
            )
            .required(false)
            .add_string_choice("Poulet (renomme @X le Poulet)", "chicken")
            .add_string_choice("Peau de banane (rate 30% des d20)", "banana")
            .add_string_choice("Portefeuille troue (10c de frais par tx)", "leaky_wallet")
            .add_string_choice("Lenteur (messages combat +10s)", "slowness")
            .add_string_choice("Insomnie (taunts defaite +50%)", "insomnia")
            .add_string_choice("Malchance amoureuse (licorne bloquee)", "heartbreak"),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }

    let target_id = match command
        .data
        .options
        .iter()
        .find(|o| o.name == "cible")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        }) {
        Some(id) => id,
        None => {
            reply_ephemeral(ctx, command, "Cible manquante.").await;
            return;
        }
    };

    let kind_opt: Option<String> = command
        .data
        .options
        .iter()
        .find(|o| o.name == "type")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        });

    let source_id = command.user.id.to_string();
    let target_id_str = target_id.to_string();

    if source_id == target_id_str {
        reply_ephemeral(ctx, command, "Tu ne peux pas te maudire toi-meme !").await;
        return;
    }

    let target_user = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };
    if target_user.bot {
        reply_ephemeral(ctx, command, "Tu ne peux pas maudire un bot !").await;
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // Garantit l existence des deux comptes (l API debit le wallet).
    if let Err(e) = api
        .get_or_create_player(&guild_id, &source_id, &command.user.name)
        .await
    {
        reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }
    if let Err(e) = api
        .get_or_create_player(&guild_id, &target_id_str, &target_user.name)
        .await
    {
        reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }

    let result = api
        .cast_curse(
            &guild_id,
            &source_id,
            &command.user.name,
            &target_id_str,
            kind_opt.as_deref(),
        )
        .await;

    match result {
        Ok(out) => {
            // Branchement Chicken : rename immediat de la cible en
            // "<pseudo> le Poulet". Reversion manuelle (le user peut
            // rename apres /lift). Best-effort.
            if out.kind == "chicken" {
                if let Some(gid) = command.guild_id {
                    crate::modules::coude::taunts_dispatch::apply_suffix_to_user(
                        ctx,
                        gid,
                        target_id,
                        " le Poulet",
                    )
                    .await;
                }
            }

            let embed = CreateEmbed::new()
                .title(format!("{} Malediction posee !", out.kind_emoji))
                .description(format!(
                    "<@{}> a maudit <@{}> avec **{}** !\n\n\
                     Duree : 24h. Cout : {}c.\n\
                     La cible peut lever en payant {}c via `/profil`.",
                    command.user.id,
                    target_id,
                    out.kind_label,
                    out.cost_paid,
                    out.cost_paid * 2,
                ))
                .color(0x9B59B6)
                .footer(CreateEmbedFooter::new(
                    sentinel_shared::branding::COUDE_TAGLINE_SHORT,
                ))
                .timestamp(serenity::model::Timestamp::now());

            crate::modules::coude::channel_check::post_activity(
                ctx,
                command,
                config.channel_activites(),
                embed,
            )
            .await;
        }
        Err(e) => {
            let msg = if e.contains("deja active") {
                "Cette cible a deja une malediction active. Attends qu elle expire.".to_string()
            } else if e.contains("toi-meme") {
                "Tu ne peux pas te maudire toi-meme !".to_string()
            } else if e.to_lowercase().contains("solde") || e.to_lowercase().contains("insufficient") {
                "Pas assez de coins (300c minimum).".to_string()
            } else {
                format!("Erreur API : {e}")
            };
            reply_ephemeral(ctx, command, &msg).await;
        }
    }
}
