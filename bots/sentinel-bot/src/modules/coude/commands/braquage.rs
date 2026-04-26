//! Commande /braquage (Phase 10).
//!
//! Permet de tenter un gros coup sur la caisse communautaire, 1 fois
//! par semaine. Base 5 % de chance, +5 % par item consommable present
//! dans l'inventaire (cap 50 %). Succes : gain 30-75 % de la caisse.
//! Echec : prison 24 h (blocage total du gameplay).
//!
//! Toute la logique metier vit cote API. Cette commande defere
//! l'interaction, appelle AttemptHeist, et affiche le resultat.

use rand::Rng;
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
};

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

const HEIST_SUCCESS: &[&str] = &[
    "\u{1f4b0} {user} a defonce la porte du coffre et s'est enfui avec {montant} coins !",
    "\u{1f3ad} Mission impossible reussie ! {user} empoche {montant} coins sans laisser de trace !",
    "\u{1f3a9} Ocean's Eleven style ! {user} sort du casino avec {montant} coins !",
    "\u{1f52b} Peaky Blinders ! {user} a vide la caisse pour {montant} coins !",
    "\u{1f3ac} Heat mode activated ! {user} rafle {montant} coins et disparait dans la nuit !",
    "\u{1f4a8} Vroum vroum ! {user} s'enfuit en bagnole avec {montant} coins !",
    "\u{1f3b7} {user} a charme la gardienne et ramasse {montant} coins comme un pro !",
    "\u{1f9e0} Plan parfait ! {user} sort de la banque avec {montant} coins, souriant !",
    "\u{1f3ab} Braquage express ! {user} repart avec {montant} coins en moins de 3 minutes !",
    "\u{1f3af} Bullseye ! {user} a fait mouche et empoche {montant} coins !",
    "\u{1f92b} Silence total ! {user} subtilise {montant} coins sans declencher l'alarme !",
    "\u{1f3a9} Maestro du crime ! {user} orchestre un casse parfait et rafle {montant} coins !",
    "\u{1f680} Braquage a la vitesse de la lumiere ! {user} emporte {montant} coins !",
    "\u{1f576}\u{fe0f} Le casse du siecle ! {user} disparait avec {montant} coins dans sa mallette !",
    "\u{1f3b2} Chance {chance}% ? Peu importe : {user} rafle {montant} coins !",
    "\u{1f47b} Fantome du coffre ! {user} emporte {montant} coins sans laisser d'empreintes !",
    "\u{1f479} Vilain genial ! {user} met la main sur {montant} coins du tresor communautaire !",
    "\u{1f9bb} Agent 007 ! {user} realise le coup parfait et rentre avec {montant} coins !",
    "\u{1f3ec} Braquage a la Bonnie & Clyde ! {user} part avec {montant} coins et du style !",
    "\u{1fa9c} Clef passe-partout ! {user} a ouvert le coffre et repart avec {montant} coins !",
];

const HEIST_FAIL: &[&str] = &[
    "\u{1f6a8} Les alarmes retentissent, {user} a tout foire et se retrouve en prison !",
    "\u{1f46e} La police a debarque ! {user} a les menottes aux poignets !",
    "\u{1fa9e} {user} a trebuche sur un cordon laser et active toutes les defenses !",
    "\u{1f436} Les chiens de garde ont repere {user} ! Direction la cellule !",
    "\u{1f3a5} Camera hyper HD : {user} en vedette du journal de 20h comme braqueur rate !",
    "\u{1f4a3} {user} a actionne la mauvaise gachette ! Explosion ! Arrete sur le champ !",
    "\u{1f9d9}\u{200d}\u{2642}\u{fe0f} Un gardien avec vision nocturne a pince {user} en plein cambriolage !",
    "\u{1fa82} {user} voulait fuir en parachute... il a oublie de l'ouvrir ! Direction prison !",
    "\u{1f91d} Le complice de {user} etait un indic ! Trahison et arrestation !",
    "\u{1f4bc} {user} a oublie son portefeuille avec son ID sur la scene du crime ! Gros indice !",
    "\u{1f3a9} Plan foireux : {user} a confondu l'entree et la sortie ! Menottes direct !",
    "\u{1f9fb} {user} a laisse une trainee d'indices comme le petit Poucet ! Rate !",
    "\u{1f4f2} Le telephone de {user} a sonne en plein braquage ! Game over !",
    "\u{1f355} {user} a commande une pizza sur les lieux du crime ! Arrestation au premier four !",
    "\u{1f57a} {user} a tente d'improviser une danse de diversion... ca n'a pas pris !",
    "\u{1f4cf} La chance etait a {chance}%, mais {user} a fait le mauvais choix a chaque etape !",
    "\u{1f32a}\u{fe0f} {user} a ete foudroye par la malchance : tentative avortee, prison !",
    "\u{1f476} {user} a pleure comme un bebe quand les sirenes ont retenti ! Capture !",
    "\u{1f3ad} Le costume de {user} est tombe au milieu du casse ! Reconnu direct !",
    "\u{1f3aa} {user} s'est pris les pieds dans la toile de tente du camouflage ! Arrete !",
];

fn pick_random<'a>(messages: &[&'a str]) -> &'a str {
    let idx = rand::thread_rng().gen_range(0..messages.len());
    messages[idx]
}

fn format_heist(template: &str, user: &str, montant: i64, chance: u32) -> String {
    template
        .replace("{user}", user)
        .replace("{montant}", &montant.to_string())
        .replace("{chance}", &chance.to_string())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("braquage")
        .description("Tente de braquer la caisse communautaire (1x par semaine, gros risque !)")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            simple_reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let config = load_guild_config(ctx, &guild_id).await;
    if !config.enabled() {
        simple_reply_ephemeral(
            ctx,
            command,
            "Le jeu Coup de Coude est desactive sur ce serveur.",
        )
        .await;
        return;
    }
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }

    // Defer : attempt_heist cote API enchaine list_inventory + get_cashbox
    // + withdraw + credit + record_attempt + (eventuellement) send_to_prison.
    // Defer public parce que le resultat est visible a tous (c'est un evt
    // du jeu, pas secret comme /protection).
    if !crate::modules::coude::interaction_helper::defer_response(ctx, command).await {
        return;
    }

    let user_id = command.user.id.to_string();

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return,
    };

    match api.attempt_heist(&guild_id, &user_id).await {
        Ok(result) => {
            let embed = build_result_embed(&command.user.id.to_string(), &result);
            crate::modules::coude::interaction_helper::followup_embed(ctx, command, embed).await;
        }
        Err(e) => {
            // L'API rejette avec DomainError::Forbidden si :
            // - en prison
            // - cooldown non ecoule
            // - caisse vide
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("\u{1f6ab} {e}")).await;
        }
    }
}

fn build_result_embed(user_id: &str, r: &crate::modules::coude::api_client::HeistResult) -> CreateEmbed {
    let user_mention = format!("<@{}>", user_id);
    if r.success {
        let tools_line = if r.tools_consumed.is_empty() {
            "Aucun outil utilise".to_string()
        } else {
            format!("{} outils consommes", r.tools_consumed.len())
        };
        let flavor = format_heist(
            pick_random(HEIST_SUCCESS),
            &user_mention,
            r.amount_stolen,
            r.chance_percent,
        );
        CreateEmbed::new()
            .title("\u{1f4b0} BRAQUAGE REUSSI !")
            .description(format!(
                "{}\n\n\
                 \u{1fa99} **+{} coins** empoches\n\
                 \u{1f3b2} Chance : **{} %**\n\
                 \u{1f6e0}\u{fe0f} {}\n\n\
                 _La caisse etait a {} coins avant le braquage._",
                flavor,
                r.amount_stolen,
                r.chance_percent,
                tools_line,
                r.cashbox_total_before
            ))
            .color(0xFFD700)
            .footer(CreateEmbedFooter::new(format!(
                "{} — Braquage hebdomadaire",
                sentinel_shared::branding::COUDE_TAGLINE_SHORT,
            )))
            .timestamp(serenity::model::Timestamp::now())
    } else {
        let prison_msg = r
            .prison_released_at
            .as_deref()
            .and_then(|ts| ts.split(&[' ', 'T'][..]).next())
            .map(|d| format!("\n\u{26d3}\u{fe0f} **EN PRISON** jusqu'au **{}** — aucune action de jeu possible !", d))
            .unwrap_or_default();

        let tools_line = if r.tools_consumed.is_empty() {
            "Aucun outil utilise".to_string()
        } else {
            format!("{} outils perdus", r.tools_consumed.len())
        };

        let flavor = format_heist(
            pick_random(HEIST_FAIL),
            &user_mention,
            r.amount_stolen,
            r.chance_percent,
        );
        CreateEmbed::new()
            .title("\u{1f6a8} BRAQUAGE RATE !")
            .description(format!(
                "{}\n\n\
                 \u{1f3b2} Chance : **{} %**\n\
                 \u{1f6e0}\u{fe0f} {}\
                 {}",
                flavor, r.chance_percent, tools_line, prison_msg
            ))
            .color(0xE74C3C)
            .footer(CreateEmbedFooter::new(format!(
                "{} — Retour dans 1 semaine minimum",
                sentinel_shared::branding::COUDE_TAGLINE_SHORT,
            )))
            .timestamp(serenity::model::Timestamp::now())
    }
}

/// Reply ephemeral simple avant defer, pour les early returns (wrong
/// channel, disabled, etc.). Utilise `create_response` classique.
async fn simple_reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    use serenity::all::{CreateInteractionResponse, CreateInteractionResponseMessage};
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord braquage");
    }
}
