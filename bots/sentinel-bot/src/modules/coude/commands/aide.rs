//! Commande `/aide` — suggestions contextuelles (cf. COUPE_AMELIORATIONS 1.3).
//!
//! Une seule commande qui repond a la question "qu est-ce que je peux faire
//! maintenant ?". Lit l etat du joueur et propose 3-6 actions pertinentes
//! triees par priorite. Aucune logique gameplay derriere — c est de l UX.

use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use sentinel_shared::discord_helpers::reply_ephemeral;

use crate::modules::coude::GameApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("aide")
        .description("Suggestions contextuelles selon l etat de ton compte")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let user_id = command.user.id.to_string();
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let player = match api
        .get_or_create_player(&guild_id, &user_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    // Construction des suggestions, par priorite decroissante.
    let mut tips: Vec<(&str, String)> = Vec::new();

    // 1. HP bas — top priorite.
    if let (Some(hp), Some(hp_max)) = (player.hp_current, player.hp_max) {
        if hp_max > 0 && hp * 4 < hp_max {
            tips.push((
                "\u{2764}\u{fe0f} HP bas",
                format!(
                    "Tu es a **{hp}/{hp_max} HP**. Tape `/repos` pour recuperer ou utilise une `/potion` si tu en as une."
                ),
            ));
        }
    }

    // 2. Stat points non depenses.
    if player.stat_points > 0 {
        tips.push((
            "\u{1f3af} Points de stat libres",
            format!(
                "Tu as **{}** points non depenses. Tape `/train` pour les placer en ATK ou DEF.",
                player.stat_points
            ),
        ));
    }

    // 3. Pas de classe choisie.
    if player.class.is_none() || player.class.as_deref() == Some("") {
        tips.push((
            "\u{2694}\u{fe0f} Choisis ta classe",
            "Tape `/classe` pour choisir entre Bourrin / Tacticien / Sournois — chacune a un passif different.".into(),
        ));
    }

    // 4. Jamais combattu.
    if player.total_wins + player.total_losses + player.total_draws == 0 {
        tips.push((
            "\u{1f44a} Premier combat",
            "Tu n as jamais combattu ! Tente un `/coude @qqun 20` (mise basse pour t echauffer).".into(),
        ));
    }

    // 5. Solde un peu maigre.
    if player.coins > 0 && player.coins < 100 {
        tips.push((
            "\u{1f4b8} Solde tres bas",
            format!(
                "Tu as seulement **{}c**. Spin `/wheel` (gratuit, 1×/jour) ou tente `/braquage` pour remonter la pente.",
                player.coins
            ),
        ));
    }

    // 6. Roue du destin daily.
    tips.push((
        "\u{1faaf} Roue du destin",
        "Tu peux spinner la roue 1× par jour avec `/wheel` — c est gratuit et ca peut taper jusqu a 10000c.".into(),
    ));

    // 7. Cagnotte.
    tips.push((
        "\u{1f3e6} Cagnotte serveur",
        "`/cagnotte` montre combien d argent dort dans la caisse — alimentee par les taxes / primes / pranks.".into(),
    ));

    // 8. Profil.
    tips.push((
        "\u{1f4cb} Ton profil",
        "`/profil` affiche tout : HP, stats, classe, malediction, assurance, inventaire.".into(),
    ));

    // 9. Theme de saison (cf. COUPE_AMELIORATIONS 6.3).
    let theme = sentinel_shared::season_theme::theme_for_season(
        player.season.unwrap_or(1),
    );
    tips.push((
        "\u{1f3ad} Saison en cours",
        format!("{} **{}** — {}", theme.emoji, theme.label, theme.tagline),
    ));

    // On limite a 6 suggestions max pour rester lisible.
    let mut embed = CreateEmbed::new()
        .title("\u{1f4a1} Aide contextuelle")
        .description(format!(
            "Voici ce que tu peux faire maintenant, <@{}> :",
            command.user.id
        ))
        .color(0x3498DB)
        .footer(CreateEmbedFooter::new(
            sentinel_shared::branding::COUDE_TAGLINE_SHORT,
        ))
        .timestamp(serenity::model::Timestamp::now());

    for (name, value) in tips.into_iter().take(6) {
        embed = embed.field(name, value, false);
    }

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response /aide");
    }
}
