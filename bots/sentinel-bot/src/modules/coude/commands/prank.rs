//! Commande `/prank` — pranks communautaires (cf. COUPE_AMELIORATIONS 5.4).
//!
//! Outils de troll pur, zero gameplay derriere, juste de l ambiance.
//! Les coins payes sont des gold sinks (debit du wallet, pas de
//! redistribution).
//!
//! Types implementes : braquage (100c), scoop (200c), appel (50c).
//! `costume` (300c) pas implemente : trop intrusif (hooker chaque
//! message d un user).

use rand::seq::SliceRandom;
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateMessage,
};

use sentinel_shared::discord_helpers::reply_ephemeral;

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

const PRANK_BRAQUAGE_COST: i64 = 100;
const PRANK_SCOOP_COST: i64 = 200;
const PRANK_APPEL_COST: i64 = 50;

const SCOOP_TEMPLATES: &[&str] = &[
    "{cible} vient de perdre 50 000 coins en voulant tout miser sur lui-meme",
    "Une source proche de {cible} confie qu il vendrait son ame contre une potion",
    "{cible} a ete vu en train de pleurer dans la cagnotte du serveur",
    "{cible} aurait depense toutes ses economies dans un boost voleur defectueux",
    "{cible} viserait une carriere de comptable selon des proches",
    "Le caissier confirme : {cible} a tente d acheter du PQ avec une carte vide",
    "{cible} aurait avoue en off vouloir abandonner Coup de Coude",
    "Selon nos sources, {cible} dort avec un poster de la cagnotte serveur",
    "{cible} aurait revele avoir 0 ami avant de jouer ici",
    "Une rumeur indique que {cible} mise tous ses coins parce qu il s ennuie",
];

const FAUX_APPEL_MESSAGES: &[&str] = &[
    "Tu as gagne 10 000 coins ! Reclame avec /claim — vite, ca expire dans 5 min !",
    "FELICITATIONS ! Tu as ete tire au sort gagnant du Tournoi Officiel ! /claim pour empocher 25 000 coins.",
    "URGENT : ton compte a ete creditee de 5 000 coins par erreur. Confirme avec /claim.",
    "Le bot t a desigene Joueur Du Mois ! Recupere ta prime de 7 500 coins via /claim.",
    "Ton boost voleur a degenere en jackpot. /claim pour debloquer 12 000 coins maintenant !",
];

pub fn register() -> CreateCommand {
    CreateCommand::new("prank")
        .description("Outils de troll communautaires (cf. roadmap 5.4)")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "type", "Type de prank")
                .required(true)
                .add_string_choice("Fausse alerte braquage (100c)", "braquage")
                .add_string_choice("Faux scoop sur un pote (200c)", "scoop")
                .add_string_choice("Faux appel en DM (50c)", "appel"),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::User,
                "cible",
                "Cible (obligatoire pour scoop / appel)",
            )
            .required(false),
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

    let prank_type = command
        .data
        .options
        .iter()
        .find(|o| o.name == "type")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let target_id_opt = command
        .data
        .options
        .iter()
        .find(|o| o.name == "cible")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        });

    let source_id = command.user.id.to_string();
    let cost = match prank_type.as_str() {
        "braquage" => PRANK_BRAQUAGE_COST,
        "scoop" => PRANK_SCOOP_COST,
        "appel" => PRANK_APPEL_COST,
        _ => {
            reply_ephemeral(ctx, command, "Type de prank inconnu.").await;
            return;
        }
    };

    // Validations dependant du type.
    let target_user = if matches!(prank_type.as_str(), "scoop" | "appel") {
        let Some(tid) = target_id_opt else {
            reply_ephemeral(ctx, command, "Ce prank necessite une cible.").await;
            return;
        };
        match tid.to_user(&ctx.http).await {
            Ok(u) if u.bot => {
                reply_ephemeral(ctx, command, "Pas de prank contre un bot.").await;
                return;
            }
            Ok(u) => Some(u),
            Err(_) => {
                reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
                return;
            }
        }
    } else {
        None
    };

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let player = match api
        .get_or_create_player(&guild_id, &source_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };
    if player.coins < cost {
        reply_ephemeral(
            ctx,
            command,
            &format!("Pas assez de coins ! Il te faut {cost}c."),
        )
        .await;
        return;
    }

    if let Err(e) = api.update_player_coins(&guild_id, &source_id, -cost).await {
        reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }

    match prank_type.as_str() {
        "braquage" => execute_braquage(ctx, command, &config).await,
        "scoop" => execute_scoop(ctx, command, &config, target_user.as_ref().unwrap()).await,
        "appel" => execute_appel(ctx, command, target_user.as_ref().unwrap()).await,
        _ => unreachable!(),
    }
}

async fn execute_braquage(
    ctx: &Context,
    command: &CommandInteraction,
    config: &crate::modules::coude::guild_config::CoudeConfig,
) {
    // Faux montant aleatoire convaincant entre 5k et 50k. RNG scope
    // ferme avant le premier await pour eviter le `Send` futur.
    let fake_amount: i64 = {
        let mut rng = rand::thread_rng();
        (rng.gen_range(5..=50)) * 1000
    };

    let embed = CreateEmbed::new()
        .title("\u{1f6a8} BRAQUAGE EN COURS !")
        .description(format!(
            "**ALERTE !** Un braquage est en cours !\n\
             La cagnotte serveur affiche **{} coins** !\n\n\
             Tout le monde sur le pont !!! \u{1f4b0}",
            fake_amount
        ))
        .color(0xE74C3C)
        .footer(CreateEmbedFooter::new(format!(
            "(prank pose par {})",
            command.user.name
        )))
        .timestamp(serenity::model::Timestamp::now());

    crate::modules::coude::channel_check::post_activity(
        ctx,
        command,
        config.channel_activites(),
        embed,
    )
    .await;
}

async fn execute_scoop(
    ctx: &Context,
    command: &CommandInteraction,
    config: &crate::modules::coude::guild_config::CoudeConfig,
    target: &serenity::model::user::User,
) {
    let tmpl: &str = {
        let mut rng = rand::thread_rng();
        SCOOP_TEMPLATES.choose(&mut rng).copied().unwrap_or("")
    };
    let body = tmpl.replace("{cible}", &format!("<@{}>", target.id));

    let embed = CreateEmbed::new()
        .title("\u{1f4f0} SCOOP")
        .description(body)
        .color(0xF1C40F)
        .footer(CreateEmbedFooter::new(format!(
            "(rumeur infondee posee par {})",
            command.user.name
        )))
        .timestamp(serenity::model::Timestamp::now());

    crate::modules::coude::channel_check::post_activity(
        ctx,
        command,
        config.channel_activites(),
        embed,
    )
    .await;
}

async fn execute_appel(
    ctx: &Context,
    command: &CommandInteraction,
    target: &serenity::model::user::User,
) {
    let tmpl: &str = {
        let mut rng = rand::thread_rng();
        FAUX_APPEL_MESSAGES
            .choose(&mut rng)
            .copied()
            .unwrap_or("Tu as gagne quelque chose !")
    };

    let embed = CreateEmbed::new()
        .title("\u{1f4de} Notification automatique")
        .description(tmpl)
        .color(0x57F287)
        .footer(CreateEmbedFooter::new(
            "Bot officiel — ce message est genere automatiquement",
        ))
        .timestamp(serenity::model::Timestamp::now());

    let dm_result = target.id.create_dm_channel(&ctx.http).await;
    let mut delivered = false;
    if let Ok(channel) = dm_result {
        if channel
            .send_message(&ctx.http, CreateMessage::new().embed(embed))
            .await
            .is_ok()
        {
            delivered = true;
        }
    }

    let confirmation = if delivered {
        format!(
            "DM envoye a <@{}> ! Attends de voir s il essaie le `/claim` qui n existe pas... \u{1f608}",
            target.id
        )
    } else {
        format!(
            "Impossible d envoyer un DM a <@{}> (DM ferme ?). Tes coins sont quand meme partis, desole.",
            target.id
        )
    };
    reply_ephemeral(ctx, command, &confirmation).await;
}

// re-exports rand pour le scope du fichier
use rand::Rng;
