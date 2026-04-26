use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use sentinel_shared::discord_helpers::reply_ephemeral;

use crate::modules::coude::catalog::CatalogCacheKey;
use crate::modules::coude::GameApiKey;
use crate::modules::coude::load_guild_config;

pub fn register() -> CreateCommand {
    CreateCommand::new("profil")
        .description("Affiche ton profil Coup de Coude")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Joueur a consulter")
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
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_profil()).await {
        return;
    }
    if !config.enabled() {
        reply_ephemeral(ctx, command, "Le jeu Coup de Coude est desactive sur ce serveur.").await;
        return;
    }

    let target_user = command
        .data
        .options
        .iter()
        .find(|o| o.name == "user")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        })
        .unwrap_or(command.user.id);

    let target = match target_user.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let catalog = data.get::<CatalogCacheKey>().unwrap().clone();

    let player = match api.get_or_create_player(&guild_id, &target.id.to_string(), &target.name).await {
        Ok(p) => p,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    // Inventaire + assurance active (best-effort : une erreur ne bloque pas l'affichage du profil).
    let inventory = api
        .get_inventory(&guild_id, &target.id.to_string())
        .await
        .unwrap_or_default();
    let active_insurance = api
        .get_active_insurance(&guild_id, &target.id.to_string())
        .await
        .ok()
        .flatten();
    let active_curse = api
        .get_active_curse(&guild_id, &target.id.to_string())
        .await
        .ok()
        .flatten();

    let class = catalog.get_class(player.class.as_deref().unwrap_or("bourrin"));
    let title = catalog.title_for_level(player.level).to_string();

    let effective_atk = class.base_atk + (player.level - 1) * class.atk_growth + player.atk;
    let effective_def = class.base_def + (player.level - 1) * class.def_growth + player.def;
    let hp = catalog.display_hp(effective_def);

    let xp_display = if player.level >= catalog.max_level {
        "MAX".to_string()
    } else {
        format!("{} / {}", player.xp, catalog.xp_for_level(player.level + 1))
    };

    let class_name_cap = {
        let mut c = class.name.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    };

    // Format inventaire : emoji + nom (x N) par ligne, trie par item_key.
    let inventory_field = if inventory.is_empty() {
        "_Aucun objet_".to_string()
    } else {
        let mut items = inventory;
        items.sort_by(|a, b| a.item_key.cmp(&b.item_key));
        items
            .iter()
            .filter(|i| i.quantity > 0)
            .map(|i| match catalog.get_item(&i.item_key) {
                Some(def) => format!("{} **{}** x{}", def.emoji, def.name, i.quantity),
                None => format!("\u{2753} {} x{}", i.item_key, i.quantity),
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let inventory_field = if inventory_field.is_empty() {
        "_Aucun objet_".to_string()
    } else {
        inventory_field
    };

    // Format assurance active : duree restante (expires_at est une string
    // RFC3339, on la parse pour calculer le delta avec now).
    let insurance_field = active_insurance.and_then(|ins| {
        chrono::DateTime::parse_from_rfc3339(&ins.expires_at)
            .ok()
            .map(|expires| {
                let remaining = expires
                    .with_timezone(&chrono::Utc)
                    .signed_duration_since(chrono::Utc::now());
                let remaining_str = if remaining.num_days() >= 1 {
                    format!("{}j {}h", remaining.num_days(), remaining.num_hours() % 24)
                } else if remaining.num_hours() >= 1 {
                    format!("{}h {}m", remaining.num_hours(), remaining.num_minutes() % 60)
                } else if remaining.num_minutes() >= 1 {
                    format!("{}m", remaining.num_minutes())
                } else {
                    "<1m".to_string()
                };
                format!("\u{1f6e1}\u{fe0f} Active — expire dans **{}**", remaining_str)
            })
    });

    let mut embed = CreateEmbed::new()
        .title(format!(
            "\u{2694}\u{fe0f} {} — {} Niv.{} \u{300c}{}\u{300d}",
            target.name, class_name_cap, player.level, title
        ))
        .color(0x3498DB)
        .thumbnail(target.face())
        .description(format!(
            "\u{2764}\u{fe0f} HP: **{}**  |  \u{2694}\u{fe0f} ATK: **{}**  |  \u{1f6e1}\u{fe0f} DEF: **{}**\n\
             \u{1fa99} **{}** coins  |  \u{1f3c6} {}W / {}L / {}D\n\
             \u{1f4ca} XP: {}  |  \u{1f3af} Points: **{}**\n\
             \u{1f414} Lachete: {}  |  \u{1f300} Chaos: {}",
            hp,
            effective_atk,
            effective_def,
            player.coins,
            player.total_wins,
            player.total_losses,
            player.total_draws,
            xp_display,
            player.stat_points,
            player.cowardice_count,
            player.chaos_events,
        ))
        .field(
            "Classe",
            format!("{} **{}** — {}", class.emoji, class.name, class.description),
            false,
        )
        .field(
            "\u{1f4b0} Gains/Pertes",
            format!("+{} / -{}", player.total_earned, player.total_lost),
            true,
        )
        .field(
            "\u{1f5e1}\u{fe0f} Total vole",
            format!("{}", player.total_stolen),
            true,
        )
        .field("\u{1f3d2} Inventaire", inventory_field, false)
        .footer(CreateEmbedFooter::new(sentinel_shared::branding::COUDE_TAGLINE_SHORT))
        .timestamp(serenity::model::Timestamp::now());

    if let Some(ins_text) = insurance_field {
        embed = embed.field("\u{1f6e1}\u{fe0f} Assurance", ins_text, false);
    }

    // Paliers visibles (cf. COUPE_AMELIORATIONS 3.2) — debloques par
    // niveau, declaratif uniquement pour l instant.
    let milestones_text =
        crate::modules::coude::milestones::format_profile_section(player.level);
    embed = embed.field("\u{1f4ca} Paliers", milestones_text, false);

    // Succes cosmetiques (cf. COUPE_AMELIORATIONS 3.4) — derives de
    // l etat actuel du joueur, aucune persistance dediee.
    let achievements_text =
        crate::modules::coude::achievements::format_unlocked_compact(&player);
    embed = embed.field("\u{1f3c5} Succes", achievements_text, false);

    // Theme de la saison courante (cf. COUPE_AMELIORATIONS 6.3) —
    // calcule deterministe depuis le numero de saison du joueur.
    let theme = sentinel_shared::season_theme::theme_for_season(
        player.season.unwrap_or(1),
    );
    embed = embed.field(
        format!("{} {}", theme.emoji, theme.label),
        theme.tagline.to_string(),
        false,
    );

    // Malediction OU sabotage actif (cf. COUPE_AMELIORATIONS 5.1 / 5.2).
    if let Some(curse) = active_curse {
        let remaining_str = chrono::DateTime::parse_from_rfc3339(&curse.expires_at)
            .ok()
            .map(|expires| {
                let remaining = expires
                    .with_timezone(&chrono::Utc)
                    .signed_duration_since(chrono::Utc::now());
                if remaining.num_days() >= 1 {
                    format!("{}j {}h", remaining.num_days(), remaining.num_hours() % 24)
                } else if remaining.num_hours() >= 1 {
                    format!("{}h {}m", remaining.num_hours(), remaining.num_minutes() % 60)
                } else if remaining.num_minutes() >= 1 {
                    format!("{}m", remaining.num_minutes())
                } else {
                    "<1m".to_string()
                }
            })
            .unwrap_or_else(|| "?".to_string());

        // Pancarte / Graisser = sabotages (format dedie).
        match curse.kind.as_str() {
            "pancarte" => {
                embed = embed.field(
                    format!("{} Rival officiel", curse.kind_emoji),
                    format!(
                        "Sous le nez de tout le monde : **Rival officiel de <@{}>**\nExpire dans **{}**.",
                        curse.source_id, remaining_str
                    ),
                    false,
                );
            }
            "graisser" => {
                embed = embed.field(
                    format!("{} Armes graissees", curse.kind_emoji),
                    format!(
                        "Sabotage de <@{}> : ta prochaine attaque speciale en combat va foirer ! Expire dans **{}**.",
                        curse.source_id, remaining_str
                    ),
                    false,
                );
            }
            _ => {
                embed = embed.field(
                    format!("{} Malediction", curse.kind_emoji),
                    format!(
                        "**{}** — pose par <@{}>\nExpire dans **{}** (lever : 600c)",
                        curse.kind_label, curse.source_id, remaining_str
                    ),
                    false,
                );
            }
        }
    }

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

