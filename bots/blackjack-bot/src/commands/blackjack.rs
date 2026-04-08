use rand::Rng;
use serenity::all::{
    ButtonStyle, CommandDataOptionValue, CommandInteraction, CommandOptionType,
    ComponentInteraction, Context, CreateActionRow, CreateButton, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::api_client::{BlackjackGameDto, CardDto};
use crate::GameApiKey;

// ── Slash command registration ──

pub fn register() -> CreateCommand {
    CreateCommand::new("blackjack")
        .description("Joue au Blackjack ! Tente d'atteindre 21 sans depasser.")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "mise", "Montant a miser")
                .required(true)
                .min_int_value(10),
        )
}

// ── Slash command handler ──

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let user_id = command.user.id.to_string();
    let username = command.user.name.clone();

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

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => {
            reply_ephemeral(ctx, command, "Erreur interne : API non disponible.").await;
            return;
        }
    };

    // Verifier s'il y a une partie en cours
    match api.get_active(&guild_id, &user_id).await {
        Ok(Some(game)) if game.status == "playing" => {
            // Reprendre la partie en cours
            let embed = build_game_embed(&game);
            let components = build_buttons(&game);
            command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .embed(embed)
                            .components(components),
                    ),
                )
                .await
                .ok();
            return;
        }
        _ => {}
    }

    // Nouvelle partie
    let game = match api.start_game(&guild_id, &user_id, &username, mise).await {
        Ok(g) => g,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let embed = build_game_embed(&game);
    let components = if is_game_over(&game.status) {
        vec![]
    } else {
        build_buttons(&game)
    };

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(components),
            ),
        )
        .await
        .ok();
}

// ── Component (button) handler ──

pub async fn handle_component(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = &component.data.custom_id;

    // Verifier que c'est le bon joueur
    let game_id = if let Some(id) = custom_id.split(':').nth(1) {
        id.to_string()
    } else {
        reply_component_ephemeral(ctx, component, "ID de partie invalide.").await;
        return;
    };

    let action = if custom_id.starts_with("bj_hit:") {
        "hit"
    } else if custom_id.starts_with("bj_stand:") {
        "stand"
    } else if custom_id.starts_with("bj_double:") {
        "double"
    } else {
        return;
    };

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => {
            reply_component_ephemeral(ctx, component, "Erreur interne.").await;
            return;
        }
    };

    // Verifier que c'est bien le proprietaire de la partie
    // On execute l'action — l'API rejettera si le game_id ne correspond pas

    let result = match action {
        "hit" => api.hit(&game_id).await,
        "stand" => api.stand(&game_id).await,
        "double" => api.double_down(&game_id).await,
        _ => return,
    };

    let game = match result {
        Ok(g) => g,
        Err(e) => {
            reply_component_ephemeral(ctx, component, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let embed = build_game_embed(&game);
    let components = if is_game_over(&game.status) {
        vec![]
    } else {
        build_buttons(&game)
    };

    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(components),
            ),
        )
        .await
        .ok();
}

// ── Helpers ──

fn is_game_over(status: &str) -> bool {
    matches!(
        status,
        "player_bust" | "player_win" | "dealer_win" | "dealer_bust" | "push" | "player_blackjack"
    )
}

fn card_to_unicode(card: &CardDto) -> String {
    if card.rank == "hidden" {
        return "🂠".to_string();
    }

    let suit_emoji = match card.suit.as_str() {
        "hearts" => "♥️",
        "diamonds" => "♦️",
        "clubs" => "♣️",
        "spades" => "♠️",
        _ => "?",
    };

    let rank_display = match card.rank.as_str() {
        "A" => "As",
        "K" => "Roi",
        "Q" => "Dame",
        "J" => "Valet",
        r => r,
    };

    format!("{}{}", rank_display, suit_emoji)
}

fn hand_to_string(hand: &[CardDto]) -> String {
    hand.iter()
        .map(card_to_unicode)
        .collect::<Vec<_>>()
        .join("  ")
}

fn build_game_embed(game: &BlackjackGameDto) -> CreateEmbed {
    let over = is_game_over(&game.status);

    let player_hand_str = hand_to_string(&game.player_hand);
    let dealer_hand_str = hand_to_string(&game.dealer_hand);

    let dealer_score_str = if over {
        format!("{}", game.dealer_score)
    } else {
        format!("{}+?", game.dealer_score)
    };

    let (title, description, color) = if !over {
        (
            "🃏 BLACKJACK".to_string(),
            format!("**Mise :** {} coins\n\nA toi de jouer !", game.bet),
            0xF1C40F, // or
        )
    } else {
        match game.status.as_str() {
            "player_blackjack" => {
                let msg = pick_random(BJ_NATURAL).replace("{joueur}", &game.username);
                (
                    "🌟 BLACKJACK NATUREL !".to_string(),
                    format!("{}\n\n**+{} coins !**", msg, game.payout),
                    0x57F287, // vert
                )
            }
            "player_win" | "dealer_bust" => {
                let msg = pick_random(BJ_WIN)
                    .replace("{joueur}", &game.username)
                    .replace("{total}", &game.player_score.to_string())
                    .replace("{croupier}", &game.dealer_score.to_string())
                    .replace("{gain}", &game.payout.to_string());
                (
                    "🎉 VICTOIRE !".to_string(),
                    format!("{}\n\n**+{} coins !**", msg, game.payout),
                    0x57F287,
                )
            }
            "player_bust" => {
                let msg = pick_random(BJ_BUST)
                    .replace("{joueur}", &game.username)
                    .replace("{total}", &game.player_score.to_string());
                let lost = if game.doubled { game.bet * 2 } else { game.bet };
                (
                    "💥 BUST !".to_string(),
                    format!("{}\n\n**-{} coins**", msg, lost),
                    0xED4245, // rouge
                )
            }
            "dealer_win" => {
                let msg = pick_random(BJ_LOSE)
                    .replace("{joueur}", &game.username)
                    .replace("{total}", &game.player_score.to_string())
                    .replace("{croupier}", &game.dealer_score.to_string())
                    .replace("{mise}", &game.bet.to_string());
                let lost = if game.doubled { game.bet * 2 } else { game.bet };
                (
                    "😤 DEFAITE".to_string(),
                    format!("{}\n\n**-{} coins**", msg, lost),
                    0xED4245,
                )
            }
            "push" => (
                "🤝 EGALITE".to_string(),
                format!(
                    "{} et le croupier font tous les deux **{}**.\nMise remboursee !",
                    game.username, game.player_score
                ),
                0x95A5A6, // gris
            ),
            _ => (
                "🃏 BLACKJACK".to_string(),
                "Partie terminee.".to_string(),
                0x95A5A6,
            ),
        }
    };

    let mut embed = CreateEmbed::new()
        .title(&title)
        .description(&description)
        .field(
            "🎴 Tes cartes",
            format!("{}\n**Score : {}**", player_hand_str, game.player_score),
            true,
        )
        .field(
            "🏦 Croupier",
            format!("{}\n**Score : {}**", dealer_hand_str, dealer_score_str),
            true,
        )
        .color(color)
        .footer(CreateEmbedFooter::new("Blackjack | Sentinel"))
        .timestamp(serenity::model::Timestamp::now());

    if game.doubled {
        embed = embed.field("💰 Mise doublee", format!("{} coins", game.bet * 2), false);
    }

    embed
}

fn build_buttons(game: &BlackjackGameDto) -> Vec<CreateActionRow> {
    let game_id = &game.id;

    let hit_btn = CreateButton::new(format!("bj_hit:{game_id}"))
        .label("Tirer")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "🃏".to_string(),
        ))
        .style(ButtonStyle::Primary);

    let stand_btn = CreateButton::new(format!("bj_stand:{game_id}"))
        .label("Rester")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "✋".to_string(),
        ))
        .style(ButtonStyle::Secondary);

    let mut buttons = vec![hit_btn, stand_btn];

    // Doubler seulement possible au premier tour (2 cartes en main)
    if game.player_hand.len() == 2 && !game.doubled {
        let double_btn = CreateButton::new(format!("bj_double:{game_id}"))
            .label("Doubler")
            .emoji(serenity::model::channel::ReactionType::Unicode(
                "💰".to_string(),
            ))
            .style(ButtonStyle::Danger);
        buttons.push(double_btn);
    }

    vec![CreateActionRow::Buttons(buttons)]
}

fn pick_random(messages: &[&str]) -> String {
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..messages.len());
    messages[idx].to_string()
}

// ── Messages fun ──

const BJ_NATURAL: &[&str] = &[
    "BLACKJACK NATUREL ! {joueur} sort 21 du premier coup ! Legendaire !",
    "21 en deux cartes ! {joueur} est un dieu du Blackjack !",
    "La perfection ! {joueur} pose un Blackjack avec classe !",
    "{joueur} claque un 21 naturel ! Le croupier en pleure !",
];

const BJ_WIN: &[&str] = &[
    "{joueur} l'emporte avec {total} contre {croupier} ! +{gain} coins !",
    "La main de maitre ! {joueur} bat le croupier {total} a {croupier} !",
    "{joueur} sourit : {total} contre {croupier}. Le croupier range ses cartes.",
    "Bien joue {joueur} ! {total} points suffisent pour terrasser le croupier ({croupier}) !",
    "{joueur} encaisse avec un {total} solide. Le croupier s'incline a {croupier}.",
];

const BJ_BUST: &[&str] = &[
    "BUST ! {joueur} a ete trop gourmand ! {total} points... c'est la cata !",
    "{joueur} depasse 21 avec {total} ! Le croupier ricane.",
    "{joueur} pensait que plus c'est haut mieux c'est... {total} points. Perdu.",
    "Aie ! {joueur} explose a {total}. La gourmandise est un vilain defaut.",
    "{joueur} tire une carte de trop et finit a {total}. Classique.",
];

const BJ_LOSE: &[&str] = &[
    "Le croupier gagne avec {croupier} contre {total}. -{mise} coins pour {joueur}.",
    "Pas de chance ! Le croupier avait {croupier}. {joueur} rage.",
    "{joueur} fait {total} mais le croupier sort {croupier}. La maison gagne toujours.",
    "Le croupier pose {croupier} avec un sourire narquois. {joueur} et ses {total} points pleurent.",
    "Dommage {joueur} ! {total} contre {croupier}. Le casino se frotte les mains.",
];

// ── Reply helpers ──

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

async fn reply_component_ephemeral(
    ctx: &Context,
    component: &ComponentInteraction,
    content: &str,
) {
    component
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
