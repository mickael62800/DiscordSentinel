//! Commande `/memorial` — Memorial des clodos (cf. COUPE_AMELIORATIONS 6.1).
//!
//! Leaderboard public des plus grosses pertes au tout-ou-rien. Lecture
//! uniquement (top 10 par defaut).

use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::shared::discord_helpers::{reply_api_err, require_guild_id};

use crate::modules::coude::GameApiKey;

const MEMORIAL_LIMIT: i64 = 10;

pub fn register() -> CreateCommand {
    CreateCommand::new("memorial")
        .description("Memorial des clodos : top 10 plus grosses pertes au tout-ou-rien")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let entries = match api.get_memorial(&guild_id, MEMORIAL_LIMIT).await {
        Ok(e) => e,
        Err(e) => {
            reply_api_err(ctx, command, e).await;
            return;
        }
    };

    let body = if entries.is_empty() {
        "_Personne n a encore tente le tout-ou-rien sur ce serveur._\n\nQuand quelqu un perdra 80% de son wallet, son nom apparaitra ici. Pour la posterite.".to_string()
    } else {
        let mut lines = Vec::with_capacity(entries.len() + 1);
        lines.push(
            "_« Ils ont tout mise. Ils ont tout perdu. Ils n auront pas l honneur d etre oublies. »_\n"
                .to_string(),
        );
        for (i, e) in entries.iter().enumerate() {
            let medal = match i {
                0 => "\u{1f947}", // gold
                1 => "\u{1f948}", // silver
                2 => "\u{1f949}", // bronze
                _ => "\u{1faa6}", // gravestone
            };
            // delta est negatif pour 'lost' -> on affiche la perte en
            // valeur absolue.
            let perte = e.delta.abs();
            lines.push(format!(
                "{} **{}** — perdu **{}** coins (mise totale : {}c)",
                medal, e.username, perte, e.mise
            ));
        }
        lines.join("\n")
    };

    let embed = CreateEmbed::new()
        .title("\u{1faa6} MEMORIAL DES CLODOS")
        .description(body)
        .color(0x424242)
        .footer(CreateEmbedFooter::new(format!(
            "Tout-ou-rien · Au nom du Pere, du Fils et du Saint Wallet · {}",
            crate::shared::branding::COUDE_TAGLINE_SHORT,
        )))
        .timestamp(serenity::model::Timestamp::now());

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response /memorial");
    }
}
