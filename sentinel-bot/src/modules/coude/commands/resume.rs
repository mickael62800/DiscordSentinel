use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use crate::shared::discord_helpers::{
    reply_api_err, reply_embed, reply_ephemeral, require_guild_id,
};

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

const MAX_EVENTS: i64 = 15;

pub fn register() -> CreateCommand {
    CreateCommand::new("resume")
        .description("Resume des derniers mouvements de coins d'un joueur")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Joueur a consulter")
                .required(false),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else {
        return;
    };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_profil())
        .await
    {
        return;
    }
    if !config.enabled() {
        reply_ephemeral(
            ctx,
            command,
            "Le jeu Coup de Coude est desactive sur ce serveur.",
        )
        .await;
        return;
    }

    let target_user_id = command
        .data
        .options
        .iter()
        .find(|o| o.name == "user")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        })
        .unwrap_or(command.user.id);

    let target = match target_user_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // Fetch les N dernieres transactions (triees DESC par created_at cote API).
    let mut txs = match api
        .get_wallet_transactions(&guild_id, &target.id.to_string(), MAX_EVENTS)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            reply_api_err(ctx, command, e).await;
            return;
        }
    };

    // Fetch le solde courant pour l'afficher en tete (plus fiable qu'un
    // balance_after potentiellement perime si une mutation est survenue
    // entre la requete transactions et maintenant).
    let player = api
        .get_or_create_player(&guild_id, &target.id.to_string(), &target.name)
        .await
        .ok();
    let current_balance = player.as_ref().map(|p| p.coins);

    if txs.is_empty() {
        let embed = CreateEmbed::new()
            .title(format!("\u{1f4dc} Resume de {}", target.name))
            .color(0xF1C40F)
            .thumbnail(target.face())
            .description(format!(
                "Aucun mouvement enregistre pour l'instant.\n\n\u{1fa99} Solde actuel : **{}** coins",
                current_balance.unwrap_or(0)
            ))
            .footer(CreateEmbedFooter::new(crate::shared::branding::COUDE_TAGLINE_SHORT))
            .timestamp(serenity::model::Timestamp::now());

        reply_embed(ctx, command, embed).await;
        return;
    }

    // L'API retourne DESC ; on remet ASC (chronologique) pour que l'historique
    // se lise du plus ancien au plus recent — plus naturel pour debuger.
    txs.reverse();

    // Le solde "de depart" affiche est celui d'avant la plus ancienne
    // transaction listee : balance_after - amount.
    let oldest = &txs[0];
    let starting_balance = oldest.balance_after - oldest.amount;
    let newest = txs.last().unwrap();
    let ending_balance = newest.balance_after;

    // Totaux gagnes / perdus sur la periode affichee (pour debug rapide).
    let total_in: i64 = txs.iter().filter(|t| t.amount > 0).map(|t| t.amount).sum();
    let total_out: i64 = txs.iter().filter(|t| t.amount < 0).map(|t| t.amount).sum();

    let mut lines: Vec<String> = Vec::with_capacity(txs.len());
    for (idx, tx) in txs.iter().enumerate() {
        let sign = if tx.amount >= 0 { "+" } else { "" };
        let date = format_date(&tx.created_at);
        let source = pretty_source(&tx.source);
        let desc = if tx.description.is_empty() {
            String::new()
        } else {
            format!(" — {}", truncate(&tx.description, 40))
        };
        lines.push(format!(
            "`{:>2}.` `{}` **{}{}** \u{2192} `{}`  _{}_{}",
            idx + 1,
            date,
            sign,
            tx.amount,
            tx.balance_after,
            source,
            desc,
        ));
    }

    // Discord limite un champ d'embed a 1024 caracteres. On joint les lignes,
    // puis on troncature propre si besoin.
    let events_field = {
        let joined = lines.join("\n");
        if joined.len() > 1024 {
            let mut s = joined.chars().take(1010).collect::<String>();
            s.push_str("\n… (tronque)");
            s
        } else {
            joined
        }
    };

    let current_line = if let Some(cur) = current_balance {
        if cur == ending_balance {
            format!("\u{1fa99} Solde actuel : **{}** coins", cur)
        } else {
            format!(
                "\u{1fa99} Solde actuel : **{}** coins _(apres {} mouvements plus recents non affiches)_",
                cur, (cur - ending_balance).abs()
            )
        }
    } else {
        format!(
            "\u{1fa99} Solde apres derniere operation : **{}** coins",
            ending_balance
        )
    };

    let embed = CreateEmbed::new()
        .title(format!("\u{1f4dc} Resume de {}", target.name))
        .color(0x3498DB)
        .thumbnail(target.face())
        .description(format!(
            "**Point de depart** (avant le 1er mouvement liste) : **{}** coins\n\
             **Apres ces {} mouvements** : **{}** coins\n\
             **Gains periode** : +{}  |  **Pertes periode** : {}\n\n\
             {}",
            starting_balance,
            txs.len(),
            ending_balance,
            total_in,
            total_out,
            current_line,
        ))
        .field(
            format!("{} derniers mouvements (chronologique)", txs.len()),
            events_field,
            false,
        )
        .footer(CreateEmbedFooter::new(
            "Coup de Coude | Sentinel — max 15 mouvements",
        ))
        .timestamp(serenity::model::Timestamp::now());

    reply_embed(ctx, command, embed).await;
}

fn format_date(iso: &str) -> String {
    // Format API : RFC3339. On tronque "2026-04-13T18:42:03.123456Z" -> "04-13 18:42".
    if let Some(t_pos) = iso.find('T') {
        let date = &iso[..t_pos];
        let rest = &iso[t_pos + 1..];
        let time = rest.get(..5).unwrap_or(rest);
        let short_date = date.get(5..).unwrap_or(date);
        return format!("{} {}", short_date, time);
    }
    iso.to_string()
}

fn pretty_source(source: &str) -> &str {
    match source {
        "coude_combat_win" => "combat gagne",
        "coude_combat_loss" => "combat perdu",
        "coude_combat_draw" => "combat egalite",
        "coude_combat_vol_bonus" => "bonus vol chaos",
        "coude_combat_bet_bonus" => "bonus paris",
        "coude_combat_expire_penalty" => "lachete",
        "coude_bet_place" => "pari place",
        "coude_bet_win" | "coude_bet_win_worker" => "pari gagne",
        "coude_bet_refund" => "pari remboursement",
        "coude_bet_unresolved_refund" | "coude_bet_expire_refund" => "pari expire",
        "coude_bet_fighter_bonus_win" => "bonus combattant",
        "coude_bet_fighter_bonus_lose" => "consolation",
        "coude_transfer_in" => "transfert recu",
        "coude_transfer_out" => "transfert envoye",
        "coude_steal_thief" => "vol reussi",
        "coude_steal_victim" => "vole par qqn",
        "coude_casino_win" => "blackjack gagne",
        "coude_casino_loss" => "blackjack perdu",
        "coude_casino_faillite" => "blackjack faillite",
        "coude_reset_stats" => "reset stats",
        "coude_earn" => "gain divers",
        "coude_loss" => "perte diverse",
        "coude_adjust" => "ajustement admin",
        "coude" => "gain coude",
        "blackjack" => "blackjack",
        "blackjack_cancel" => "blackjack annule",
        "reset" => "reset admin",
        other => other,
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max - 1).collect::<String>() + "\u{2026}"
    }
}
