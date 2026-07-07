use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use crate::shared::discord_helpers::{reply_api_err, reply_embed, reply_ephemeral};
use crate::shared::parsers::format_duration_remaining;

use crate::modules::coude::catalog::CatalogCacheKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("profil")
        .description("Affiche ton profil Coup de Coude")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Joueur a consulter")
                .required(false),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some((guild_id, _config, api)) = crate::modules::coude::command_prelude::coude_prelude(
        ctx,
        command,
        |c| c.channel_profil(),
        true,
    )
    .await
    else {
        return;
    };

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

    let catalog = {
        let data = ctx.data.read().await;
        data.get::<CatalogCacheKey>().unwrap().clone()
    };

    let player = match api
        .get_or_create_player(&guild_id, &target.id.to_string(), &target.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_api_err(ctx, command, e).await;
            return;
        }
    };

    // Inventaire + assurance active (best-effort : une erreur ne bloque pas l'affichage du profil).
    // Les 4 lectures sont independantes → on les lance en parallele (1 latence au lieu de 4).
    let tid = target.id.to_string();
    let (inventory, active_insurance, active_curse, tor_stats, progression) = tokio::join!(
        api.get_inventory(&guild_id, &tid),
        api.get_active_insurance(&guild_id, &tid),
        api.get_active_curse(&guild_id, &tid),
        api.get_user_tout_ou_rien_stats(&guild_id, &tid),
        // Progression (succes + paliers) resolue server-side : le bareme ne
        // vit plus dans le bot, qui ne fait que rendre les emojis/labels.
        api.get_progression(&guild_id, &tid),
    );
    let inventory = inventory.unwrap_or_default();
    let active_insurance = active_insurance.ok().flatten();
    let active_curse = active_curse.ok().flatten();
    let tor_stats = tor_stats.unwrap_or_default();
    let progression = progression.ok();

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
        format_duration_remaining(&ins.expires_at).map(|remaining_str| {
            format!(
                "\u{1f6e1}\u{fe0f} Active — expire dans **{}**",
                remaining_str
            )
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
        .footer(CreateEmbedFooter::new(crate::shared::branding::COUDE_TAGLINE_SHORT))
        .timestamp(serenity::model::Timestamp::now());

    if let Some(ins_text) = insurance_field {
        embed = embed.field("\u{1f6e1}\u{fe0f} Assurance", ins_text, false);
    }

    // Paliers visibles (cf. COUPE_AMELIORATIONS 3.2) — la liste + le statut de
    // deblocage viennent de l'API (le bot ne porte plus la table MILESTONES).
    let milestones_text = progression
        .as_ref()
        .map(crate::modules::coude::milestones::format_profile_section)
        .unwrap_or_else(|| "_Indisponible._".to_string());
    embed = embed.field("\u{1f4ca} Paliers", milestones_text, false);

    // Stats /tout-ou-rien (cf. COUPE_AMELIORATIONS 6.1) — affichees
    // uniquement si le joueur a deja tente au moins une fois.
    if tor_stats.attempts > 0 {
        let win_rate = if tor_stats.attempts > 0 {
            (tor_stats.wins as f64 / tor_stats.attempts as f64 * 100.0) as i64
        } else {
            0
        };
        let mut lines = vec![format!(
            "**{}** tentative{} — {} victoire{} / {} defaite{} ({}% de win-rate)",
            tor_stats.attempts,
            if tor_stats.attempts > 1 { "s" } else { "" },
            tor_stats.wins,
            if tor_stats.wins > 1 { "s" } else { "" },
            tor_stats.losses,
            if tor_stats.losses > 1 { "s" } else { "" },
            win_rate,
        )];
        if tor_stats.biggest_win > 0 {
            lines.push(format!(
                "\u{1f3c6} Plus gros gain : **+{}** coins",
                tor_stats.biggest_win
            ));
        }
        if tor_stats.biggest_loss > 0 {
            lines.push(format!(
                "\u{1faa6} Plus grosse perte : **-{}** coins (au Memorial)",
                tor_stats.biggest_loss
            ));
        }
        embed = embed.field("\u{1f3b2} Tout-ou-rien", lines.join("\n"), false);
    }

    // Succes cosmetiques (cf. COUPE_AMELIORATIONS 3.4) — la liste debloquee est
    // derivee server-side (bareme cote API), le bot ne fait que l'affichage.
    let achievements_text = progression
        .as_ref()
        .map(crate::modules::coude::achievements::format_unlocked_compact)
        .unwrap_or_else(|| "_Indisponible._".to_string());
    embed = embed.field("\u{1f3c5} Succes", achievements_text, false);

    // Theme de la saison courante (cf. COUPE_AMELIORATIONS 6.3) —
    // calcule deterministe depuis le numero de saison du joueur.
    let theme = crate::shared::season_theme::theme_for_season(player.season.unwrap_or(1));
    embed = embed.field(
        format!("{} {}", theme.emoji, theme.label),
        theme.tagline.to_string(),
        false,
    );

    // Malediction OU sabotage actif (cf. COUPE_AMELIORATIONS 5.1 / 5.2).
    if let Some(curse) = active_curse {
        let remaining_str =
            format_duration_remaining(&curse.expires_at).unwrap_or_else(|| "?".to_string());

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
            "empoisonner" => {
                embed = embed.field(
                    format!("{} Wallet empoisonne", curse.kind_emoji),
                    format!(
                        "Sabotage de <@{}> : 10% de tes prochains gains de combat sont redirige vers lui. Expire dans **{}**.",
                        curse.source_id, remaining_str
                    ),
                    false,
                );
            }
            "fausse_assurance" => {
                embed = embed.field(
                    format!("{} Fausse assurance", curse.kind_emoji),
                    format!(
                        "Sabotage de <@{}> : ton prochain combat perdu avec une assurance, la protection ne s applique pas + 200c preleves. Expire dans **{}**.",
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

    reply_embed(ctx, command, embed).await;
}
