//! Commande `/transfert` — convertit un capital en un autre (04.md §10).
//!
//! « Le jeu consiste a transformer un capital en un autre. » Conversions
//! autorisees : Argent -> Reputation, Reputation -> Influence, Argent ->
//! Information. Les taux sont regles via la config du serveur.

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed,
};

use crate::modules::influence::api_client;
use crate::shared::discord_helpers::{option_i64, option_str, reply_ephemeral, reply_ephemeral_embed, require_guild_id};
use crate::shared::heartbeat::ApiClientKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("transfert")
        .description("Convertit un capital en un autre")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "conversion", "Type de conversion")
                .required(true)
                .add_string_choice("Argent → Réputation", "money_reputation")
                .add_string_choice("Réputation → Influence", "reputation_influence")
                .add_string_choice("Argent → Information", "money_information"),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "montant",
                "Montant du capital source a investir",
            )
            .required(true)
            .min_int_value(1),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };
    let kind = option_str(&command.data.options, "conversion").unwrap_or("");
    let budget = option_i64(&command.data.options, "montant").unwrap_or(0);

    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    match api_client::convert_capital(
        &api,
        &guild_id,
        &command.user.id.to_string(),
        &command.user.name,
        kind,
        budget,
    )
    .await
    {
        Ok(o) => {
            let embed = CreateEmbed::new()
                .title("🔁 Conversion réussie")
                .color(0x2ECC71)
                .description(format!(
                    "Tu as investi **{} {}** pour obtenir **+{} {}**.",
                    o.spent, o.source_label, o.gained, o.target_label
                ))
                .field(format!("{} restant", o.source_label), o.new_source.to_string(), true)
                .field(format!("{} total", o.target_label), o.new_target.to_string(), true);
            reply_ephemeral_embed(ctx, command, embed).await;
        }
        Err(e) => reply_ephemeral(ctx, command, &format!("Conversion impossible : {e}")).await,
    }
}
