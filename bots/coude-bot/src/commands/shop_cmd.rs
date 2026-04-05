use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::game::shop::{self, SHOP_ITEMS};
use crate::handler::{GameDbKey, load_guild_config};

pub fn register() -> CreateCommand {
    CreateCommand::new("shop")
        .description("Boutique Coup de Coude")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "acheter", "Objet a acheter")
                .required(false)
                .add_string_choice("Explosion (200)", "explosion")
                .add_string_choice("Inversion (500)", "inversion")
                .add_string_choice("Mindgame (150)", "mindgame")
                .add_string_choice("Rage (100)", "rage")
                .add_string_choice("Attaque surprise (300)", "surprise")
                .add_string_choice("Double coup (250)", "double_coup")
                .add_string_choice("Coup traitre (350)", "coup_traitre"),
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

    let buy_key = command
        .data
        .options
        .iter()
        .find(|o| o.name == "acheter")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        });

    let config = load_guild_config(ctx, &guild_id).await;

    let data = ctx.data.read().await;
    let db = data.get::<GameDbKey>().unwrap();

    match buy_key {
        Some(key) => {
            // Achat d'un objet
            let item = match shop::get_item(&key) {
                Some(i) => i,
                None => {
                    reply_ephemeral(ctx, command, "Objet inconnu.").await;
                    return;
                }
            };

            let player = match db
                .get_or_create_player(&guild_id, &command.user.id.to_string(), &command.user.name)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
                    return;
                }
            };

            if player.coins < item.price {
                reply_ephemeral(
                    ctx,
                    command,
                    &format!(
                        "Pas assez de coins ! Tu as {} coins, il en faut {}.",
                        player.coins, item.price
                    ),
                )
                .await;
                return;
            }

            // Deduire les coins
            if let Err(e) = db
                .update_player_coins(&guild_id, &command.user.id.to_string(), -item.price)
                .await
            {
                reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
                return;
            }

            // Ajouter l'item
            if let Err(e) = db
                .add_item(&guild_id, &command.user.id.to_string(), &key)
                .await
            {
                reply_ephemeral(ctx, command, &format!("Erreur DB : {e}")).await;
                return;
            }

            let embed = CreateEmbed::new()
                .title(format!("{} Achat reussi !", item.emoji))
                .description(format!(
                    "<@{}> a achete **{} {}** pour **{} coins** !\n\n_{}_",
                    command.user.id, item.emoji, item.name, item.price, item.description
                ))
                .color(0x3498DB)
                .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
                .timestamp(serenity::model::Timestamp::now());

            command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().embed(embed),
                    ),
                )
                .await
                .ok();
        }
        None => {
            // Afficher la boutique
            let mut desc = String::from("Utilise `/shop acheter:<item>` pour acheter !\n\n");

            for item in SHOP_ITEMS {
                desc.push_str(&format!(
                    "{} **{}** — **{} coins**\n> _{}_\n\n",
                    item.emoji, item.name, item.price, item.description
                ));
            }

            // Afficher l'inventaire du joueur
            let inventory = db
                .get_inventory(&guild_id, &command.user.id.to_string())
                .await
                .unwrap_or_default();

            if !inventory.is_empty() {
                desc.push_str("---\n\u{1f392} **Ton inventaire :**\n");
                for inv_item in &inventory {
                    let label = shop::get_item(&inv_item.item_key)
                        .map(|i| format!("{} {}", i.emoji, i.name))
                        .unwrap_or_else(|| inv_item.item_key.clone());
                    desc.push_str(&format!("  {} x{}\n", label, inv_item.quantity));
                }
            }

            let embed = CreateEmbed::new()
                .title("\u{1f6d2} Boutique Coup de Coude")
                .description(desc)
                .color(0x3498DB)
                .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
                .timestamp(serenity::model::Timestamp::now());

            command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().embed(embed),
                    ),
                )
                .await
                .ok();
        }
    }
}

async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
        .ok();
}
