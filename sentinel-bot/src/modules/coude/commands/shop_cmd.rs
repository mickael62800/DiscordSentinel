//! Commande /shop avec sous-commandes attaque / defense.
//!
//! `/shop attaque` liste les items offensifs, `/shop defense` liste les
//! items defensifs et soins. Chaque sous-commande accepte un argument
//! optionnel `acheter:<item>` qui permet d'acheter directement, avec
//! les choices filtrees sur la categorie correspondante.
//!
//! La categorisation vit dans le domain API (`shop.rs::ShopItem.category`).
//! Le bot filtre la liste cachee du catalog.

use serenity::all::{
    CommandDataOption, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::shared::discord_helpers::{reply_api_err, reply_ephemeral, require_guild_id};

use crate::modules::coude::catalog::CatalogCacheKey;
use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

pub fn register() -> CreateCommand {
    // Sous-commande attaque — liste des items offensifs.
    // Les choices sont hardcodes ici parce que Discord exige un set
    // statique a l'enregistrement de la slash command ; la source de
    // verite reste le domain cote API, on synchronise manuellement.
    let attaque = CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "attaque",
        "Items offensifs (rage, mindgame, poison, surprise, coup traitre, double coup)",
    )
    .add_sub_option(
        CreateCommandOption::new(
            CommandOptionType::String,
            "acheter",
            "Item a acheter (facultatif : laisse vide pour juste voir la liste)",
        )
        .required(false)
        .add_string_choice("Rage (100)", "rage")
        .add_string_choice("Mindgame (150)", "mindgame")
        .add_string_choice("Double Coup (250)", "double_coup")
        .add_string_choice("Poison (300)", "poison")
        .add_string_choice("Attaque Surprise (300)", "surprise")
        .add_string_choice("Coup Traitre (350)", "coup_traitre"),
    );

    // Sous-commande defense — potions, antidote, bouclier, explosion.
    let defense = CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "defense",
        "Items defensifs et soins (potions, antidote, bouclier, explosion)",
    )
    .add_sub_option(
        CreateCommandOption::new(
            CommandOptionType::String,
            "acheter",
            "Item a acheter (facultatif)",
        )
        .required(false)
        .add_string_choice("Potion de Soin (80)", "potion_soin")
        .add_string_choice("Antidote (150)", "antidote")
        .add_string_choice("Potion Majeure (200)", "potion_majeure")
        .add_string_choice("Explosion (200)", "explosion")
        .add_string_choice("Bouclier (250)", "bouclier"),
    );

    // Sous-commande braquage (Phase 10) — items consommables pour
    // /braquage. Chacun donne +5 % au roll (cap 50 % avec les 9).
    let braquage = CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "braquage",
        "Items consommables pour /braquage (cap 50% avec les 9)",
    )
    .add_sub_option(
        CreateCommandOption::new(
            CommandOptionType::String,
            "acheter",
            "Item a acheter (facultatif)",
        )
        .required(false)
        .add_string_choice("Masque +2% (100)", "masque_braquage")
        .add_string_choice("Pied-de-biche +3% (150)", "pied_de_biche")
        .add_string_choice("Crochet de vault +4% (220)", "crochet_vault")
        .add_string_choice("Plan du coffre +5% (320)", "plan_coffre")
        .add_string_choice("Fumigene +5% (450)", "fumigene_diversion")
        .add_string_choice("Explosif +6% (600)", "explosif")
        .add_string_choice("Hacker kit +7% (800)", "hacker_kit")
        .add_string_choice("Drone espion +8% (1000)", "drone_espion")
        .add_string_choice("Equipe de pros +10% (1500)", "equipe_de_pros"),
    );

    CreateCommand::new("shop")
        .description("Boutique Coup de Coude — attaque, defense ou braquage")
        .add_option(attaque)
        .add_option(defense)
        .add_option(braquage)
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };

    // Detecte la sous-commande choisie (attaque/defense/braquage) et
    // l'argument optionnel `acheter`.
    let (category, buy_key) = match extract_subcommand(&command.data.options) {
        Some(p) => p,
        None => {
            reply_ephemeral(
                ctx,
                command,
                "Choisis une sous-commande : `/shop attaque`, `/shop defense` ou `/shop braquage`.",
            )
            .await;
            return;
        }
    };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_profil())
        .await
    {
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let catalog = data.get::<CatalogCacheKey>().unwrap().clone();

    match buy_key {
        Some(key) => {
            // ── Chemin achat ──
            let item = match catalog.get_item(&key) {
                Some(i) => i.clone(),
                None => {
                    reply_ephemeral(ctx, command, "Objet inconnu.").await;
                    return;
                }
            };

            // Safety : empeche d'acheter un item d'une autre categorie
            // via un copier-coller de choice. L'API ne connait pas la
            // notion de "categorie du shop" — c'est une regle UI,
            // documentee comme telle.
            if item.category != category {
                reply_ephemeral(
                    ctx,
                    command,
                    &format!(
                        "Cet item n'est pas dans la categorie **{}**. Utilise la bonne sous-commande.",
                        category
                    ),
                )
                .await;
                return;
            }

            let player = match api
                .get_or_create_player(&guild_id, &command.user.id.to_string(), &command.user.name)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    reply_api_err(ctx, command, e).await;
                    return;
                }
            };

            let price = config.shop_price(&item.key);

            if player.coins < price {
                reply_ephemeral(
                    ctx,
                    command,
                    &format!(
                        "Pas assez de coins ! Tu as {} coins, il en faut {}.",
                        player.coins, price
                    ),
                )
                .await;
                return;
            }

            // Deduire les coins
            if let Err(e) = api
                .update_player_coins(&guild_id, &command.user.id.to_string(), -price)
                .await
            {
                reply_api_err(ctx, command, e).await;
                return;
            }

            // Ajouter l'item — rollback si l'add_item echoue.
            if let Err(e) = api
                .add_item(&guild_id, &command.user.id.to_string(), &key)
                .await
            {
                if let Err(e2) = api
                    .update_player_coins(&guild_id, &command.user.id.to_string(), price)
                    .await
                {
                    tracing::error!(
                        error = %e2,
                        user = %command.user.id,
                        price,
                        "Echec rollback coins apres echec add_item shop : coins perdus"
                    );
                }
                reply_api_err(ctx, command, e).await;
                return;
            }

            // Phase 9 : depot caisse communautaire.
            if let Err(e) = api
                .deposit_cashbox(
                    &guild_id,
                    price,
                    crate::modules::coude::api_client::CashboxDepositSource::ShopPurchase,
                )
                .await
            {
                tracing::warn!(error = %e, guild_id, "Echec deposit cashbox shop");
            }

            let embed = CreateEmbed::new()
                .title(format!("{} Achat reussi !", item.emoji))
                .description(format!(
                    "<@{}> a achete **{} {}** pour **{} coins** !\n\n_{}_",
                    command.user.id, item.emoji, item.name, price, item.description
                ))
                .color(category_color(category))
                .footer(CreateEmbedFooter::new(
                    crate::shared::branding::COUDE_TAGLINE_SHORT,
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
        None => {
            // ── Chemin affichage : liste filtree par categorie ──
            let title = match category {
                "attaque" => "\u{2694}\u{fe0f} Boutique — Attaque",
                "defense" => "\u{1f6e1}\u{fe0f} Boutique — Defense",
                "braquage" => "\u{1f3ad} Boutique — Braquage",
                _ => "\u{1f6d2} Boutique Coup de Coude",
            };

            let intro = format!(
                "Utilise `/shop {} acheter:<item>` pour acheter !\n\n",
                category
            );
            let mut desc = intro;

            let items: Vec<_> = catalog
                .shop_items
                .iter()
                .filter(|i| i.category == category)
                .collect();

            if items.is_empty() {
                desc.push_str("_Aucun item dans cette categorie._\n\n");
            } else {
                for item in &items {
                    let price = config.shop_price(&item.key);
                    desc.push_str(&format!(
                        "{} **{}** — **{} coins**\n> _{}_\n\n",
                        item.emoji, item.name, price, item.description
                    ));
                }
            }

            // Inventaire : on affiche seulement les items de cette categorie
            // pour que la vue soit coherente avec la sous-commande choisie.
            let inventory = api
                .get_inventory(&guild_id, &command.user.id.to_string())
                .await
                .unwrap_or_default();

            let inv_in_cat: Vec<_> = inventory
                .iter()
                .filter(|inv_item| {
                    catalog
                        .get_item(&inv_item.item_key)
                        .map(|def| def.category == category)
                        .unwrap_or(false)
                })
                .collect();

            if !inv_in_cat.is_empty() {
                desc.push_str("---\n\u{1f392} **Ton inventaire (categorie) :**\n");
                for inv_item in &inv_in_cat {
                    let label = catalog
                        .get_item(&inv_item.item_key)
                        .map(|i| format!("{} {}", i.emoji, i.name))
                        .unwrap_or_else(|| inv_item.item_key.clone());
                    desc.push_str(&format!("  {} x{}\n", label, inv_item.quantity));
                }
            }

            let embed = CreateEmbed::new()
                .title(title)
                .description(desc)
                .color(category_color(category))
                .footer(CreateEmbedFooter::new(
                    crate::shared::branding::COUDE_TAGLINE_SHORT,
                ))
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
                tracing::warn!(error = %e, "Echec response Discord");
            }
        }
    }
}

/// Extrait `(category, buy_key)` depuis la sous-commande choisie.
/// Retourne `None` si aucune sous-commande valide n'a ete donnee.
fn extract_subcommand(options: &[CommandDataOption]) -> Option<(&'static str, Option<String>)> {
    for opt in options {
        let category: &'static str = match opt.name.as_str() {
            "attaque" => "attaque",
            "defense" => "defense",
            "braquage" => "braquage",
            _ => continue,
        };
        let buy_key = match &opt.value {
            CommandDataOptionValue::SubCommand(sub_opts) => sub_opts
                .iter()
                .find(|s| s.name == "acheter")
                .and_then(|s| match &s.value {
                    CommandDataOptionValue::String(v) => Some(v.clone()),
                    _ => None,
                }),
            _ => None,
        };
        return Some((category, buy_key));
    }
    None
}

fn category_color(category: &str) -> u32 {
    match category {
        "attaque" => 0xE74C3C,  // rouge
        "defense" => 0x3498DB,  // bleu
        "braquage" => 0xFFD700, // or
        _ => 0x95A5A6,
    }
}
