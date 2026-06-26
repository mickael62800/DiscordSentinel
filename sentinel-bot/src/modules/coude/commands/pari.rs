use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id};

use crate::modules::coude::GameApiKey;
use crate::modules::coude::load_guild_config;

pub fn register() -> CreateCommand {
    CreateCommand::new("pari")
        .description("Parie sur l'issue du combat d'un joueur !")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::User,
                "combattant",
                "Le joueur sur lequel tu paries (doit avoir un combat en attente)",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "mise", "Montant du pari")
                .required(true)
                .min_int_value(1),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_combats()).await {
        return;
    }
    if !config.enabled() {
        reply_ephemeral(ctx, command, "Le jeu Coup de Coude est desactive sur ce serveur.").await;
        return;
    }

    let target_id = command
        .data
        .options
        .iter()
        .find(|o| o.name == "combattant")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        })
        .unwrap();

    let mise = command
        .data
        .options
        .iter()
        .find(|o| o.name == "mise")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(10);

    let bettor_id = command.user.id.to_string();
    let target_id_str = target_id.to_string();

    // On ne peut pas parier sur soi-meme
    if bettor_id == target_id_str {
        reply_ephemeral(ctx, command, "Tu ne peux pas parier sur toi-meme !").await;
        return;
    }

    // Defer public : 3 appels API (get_player, get_betting_combat, place_bet)
    // avant la reponse — borderline 3s sans defer.
    if !crate::modules::coude::interaction_helper::defer_response(ctx, command).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // Creer/recuperer le parieur
    let bettor = match api
        .get_or_create_player(&guild_id, &bettor_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &e).await;
            return;
        }
    };

    if bettor.coins < mise {
        crate::modules::coude::interaction_helper::followup_text(
            ctx,
            command,
            &format!(
                "Pas assez de coins ! Tu as {} coins, mise demandee : {}.",
                bettor.coins, mise
            ),
        )
        .await;
        return;
    }

    // Chercher un combat en phase de paris pour le combattant
    let combat = match api
        .get_betting_combat_for_player(&guild_id, &target_id_str)
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => {
            crate::modules::coude::interaction_helper::followup_text(
                ctx,
                command,
                &format!("<@{}> n'a aucun combat ouvert aux paris !", target_id),
            )
            .await;
            return;
        }
        Err(e) => {
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &e).await;
            return;
        }
    };

    // Verifier que le parieur n'est ni attaquant ni defenseur
    if bettor_id == combat.attacker_id || bettor_id == combat.defender_id {
        crate::modules::coude::interaction_helper::followup_text(
            ctx,
            command,
            "Tu ne peux pas parier sur un combat dans lequel tu participes !",
        )
        .await;
        return;
    }

    // place_bet est atomique cote API : SELECT FOR UPDATE + UPDATE debit +
    // INSERT dans une seule transaction sur user_wallets. Pas de debit
    // upfront cote bot (sinon double-debit : une fois par update_player_coins,
    // une fois par le tx interne de place_bet).
    // Migration #7 : place_bet retourne les TauntEvents declenches
    // (faillite parieur si solde passe a zero). On les dispatche apres
    // confirmation du pari.
    let taunts = match api
        .place_bet(
            &guild_id,
            &combat.id,
            &bettor_id,
            &command.user.name,
            &target_id_str,
            mise,
        )
        .await
    {
        Ok(events) => events,
        Err(e) => {
            crate::modules::coude::interaction_helper::followup_text(
                ctx,
                command,
                &format!("Erreur pari : {e}"),
            )
            .await;
            return;
        }
    };

    let embed = CreateEmbed::new()
        .title("\u{1f3b2} Pari enregistre !")
        .description(format!(
            "<@{}> parie **{} coins** sur la victoire de <@{}> !\n\nCombat : <@{}> vs <@{}>",
            command.user.id, mise, target_id, combat.attacker_id, combat.defender_id
        ))
        .color(0xF1C40F)
        .footer(CreateEmbedFooter::new(crate::shared::branding::COUDE_TAGLINE_SHORT))
        .timestamp(serenity::model::Timestamp::now());

    crate::modules::coude::interaction_helper::followup_embed(ctx, command, embed).await;

    // Migration #7 : dispatch des taunts (faillite parieur si debit de mise
    // a vide son wallet).
    if !taunts.is_empty() {
        if let Some(gid) = command.guild_id {
            crate::modules::coude::taunts_dispatch::dispatch_all(ctx, gid, &taunts).await;
        }
    }
}

