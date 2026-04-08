use serenity::all::{
    ComponentInteraction, Context, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::id::ChannelId;
use tracing::error;

use crate::api_client::Combat;
use crate::game::chaos::ChaosEvent;
use crate::game::combat;
use crate::game::progression;
use crate::GameApiKey;
use crate::handler::load_guild_config;

pub const ACCEPT_PREFIX: &str = "coude_accept:";

pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let combat_id = match component.data.custom_id.strip_prefix(ACCEPT_PREFIX) {
        Some(id) => id.to_string(),
        None => return,
    };

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let combat_record = match api.get_combat(&combat_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            reply_ephemeral(ctx, component, "Combat introuvable.").await;
            return;
        }
        Err(e) => {
            reply_ephemeral(ctx, component, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    // Verifier que c'est bien le defenseur qui clique
    if component.user.id.to_string() != combat_record.defender_id {
        reply_ephemeral(ctx, component, "Seul le defenseur peut accepter le defi !").await;
        return;
    }

    // Verifier le statut
    if combat_record.status != "pending" {
        reply_ephemeral(ctx, component, "Ce combat n'est plus en attente.").await;
        return;
    }

    // Charger la config
    drop(data);
    let config = load_guild_config(ctx, &combat_record.guild_id).await;

    // Verifier l'expiration (configurable, defaut 24h)
    let expire_secs = config.combat_expire_secs() as i64;
    let created = chrono::DateTime::parse_from_rfc3339(&combat_record.created_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let elapsed = chrono::Utc::now()
        .signed_duration_since(created)
        .num_seconds();
    if elapsed > expire_secs {
        let data = ctx.data.read().await;
        let api = data.get::<GameApiKey>().unwrap();
        if let Err(e) = api.expire_combat(&combat_id).await {
            tracing::warn!(error = %e, "Echec API expire_combat");
        }
        let expire_label = if expire_secs >= 3600 { format!("{}h", expire_secs / 3600) } else { format!("{}min", expire_secs / 60) };
        reply_ephemeral(ctx, component, &format!("Ce defi a expire ! ({})", expire_label)).await;
        return;
    }

    let delay_min = config.bet_delay_secs() / 60;

    // Passer le combat en phase "betting" avec le message_id
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let message_id = component.message.id.to_string();

    match api.set_combat_betting(&combat_id, &message_id).await {
        Ok(false) => {
            reply_ephemeral(ctx, component, "Ce combat n'est plus en attente.").await;
            return;
        }
        Err(e) => {
            reply_ephemeral(ctx, component, &format!("Erreur API : {e}")).await;
            return;
        }
        Ok(true) => {}
    }

    // Remplacer le message de defi par "Combat accepte, paris ouverts"
    let waiting_embed = CreateEmbed::new()
        .title("\u{270a} Combat accepte !")
        .description(format!(
            "<@{}> a accepte le defi de <@{}> !\n\n\
            \u{1f3b2} **Les paris sont ouverts pendant {} minute(s) !**\n\
            Utilisez `/pari` pour miser sur le vainqueur.\n\n\
            \u{23f3} Le combat sera resolu automatiquement par le serveur...",
            combat_record.defender_id,
            combat_record.attacker_id,
            delay_min,
        ))
        .field("Mise", format!("{} coins", combat_record.mise), true)
        .color(0x3498DB)
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
        .timestamp(serenity::model::Timestamp::now());

    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(waiting_embed)
                    .components(vec![]),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }

    // Notification dans le salon notifications
    if let Some(notif_ch) = config.channel_notifications() {
        if let Ok(ch_id) = notif_ch.parse::<u64>() {
            let combat_channel = config.channel_combats().unwrap_or_default();
            let notif_embed = CreateEmbed::new()
                .title("\u{1f3b0} Paris ouverts !")
                .description(format!(
                    "**{}** vs **{}** pour **{} coins** !\n\n\
                    \u{23f3} Paris ouverts pendant **{} minute(s)** !\n\
                    Utilisez `/pari` dans <#{}> pour miser.",
                    combat_record.attacker_name, combat_record.defender_name,
                    combat_record.mise, delay_min, combat_channel,
                ))
                .color(0x57F287)
                .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
                .timestamp(serenity::model::Timestamp::now());

            if let Err(e) = serenity::model::id::ChannelId::new(ch_id)
                .send_message(&ctx.http, CreateMessage::new().embed(notif_embed))
                .await
            {
                tracing::warn!(error = %e, "Echec send_message salon notifications");
            }
        }
    }
}

/// Resoud le combat et met a jour la base de donnees.
/// Retourne un embed de resultat ou None en cas d'erreur.
/// Utilise aussi par coude.rs pour les attaques surprise.
pub async fn resolve_combat_internal(
    ctx: &Context,
    combat_record: &Combat,
    _channel_id: ChannelId,
) -> Option<CreateEmbed> {
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let attacker = match api
        .get_player(&combat_record.guild_id, &combat_record.attacker_id)
        .await
    {
        Ok(Some(p)) => p,
        _ => return None,
    };

    let defender = match api
        .get_player(&combat_record.guild_id, &combat_record.defender_id)
        .await
    {
        Ok(Some(p)) => p,
        _ => return None,
    };

    let events = api
        .get_active_events(&combat_record.guild_id)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Echec API get_active_events");
            vec![]
        });

    let atk_hp_max = combat::calculate_hp_max(&attacker);
    let def_hp_max = combat::calculate_hp_max(&defender);

    let result = combat::resolve_combat(
        &attacker,
        &defender,
        atk_hp_max,
        def_hp_max,
        combat_record.mise,
        combat_record.special_attack.as_deref(),
        combat_record.defender_special.as_deref(),
        &events,
    );

    // Mettre a jour la base de donnees
    // Aggregate chaos info from rounds
    let first_chaos = result.rounds.iter().find_map(|r| r.chaos_event);
    let chaos_key = first_chaos.as_ref().map(|c| c.key());

    // Use first round rolls for DB compat, or 0 if no rounds
    let first_atk_roll = result.rounds.first().map(|r| r.attacker_roll).unwrap_or(0);
    let first_def_roll = result.rounds.first().map(|r| r.defender_roll).unwrap_or(0);

    if let Err(e) = api
        .resolve_combat(
            &combat_record.id,
            "accepted",
            result.winner_id.as_deref(),
            Some(first_atk_roll),
            Some(first_def_roll),
            chaos_key,
            &result.message,
            result.coins_won,
        )
        .await
    {
        error!(error = %e, "Erreur resolution combat API");
        return None;
    }

    // Gestion des gains/pertes selon le resultat
    match (&result.winner_id, &result.loser_id) {
        (Some(winner_id), Some(loser_id)) => {
            // Verifier si le perdant a une assurance active
            let mut actual_loss = result.coins_lost_by_loser;
            let mut insurance_msg: Option<String> = None;

            if let Ok(Some(insurance)) = api
                .get_active_insurance(&combat_record.guild_id, loser_id)
                .await
            {
                // Consommer l'assurance
                if let Err(e) = api.expire_insurance(&insurance.id).await {
                    tracing::warn!(error = %e, "Echec API expire_insurance");
                }

                if insurance.is_scam {
                    // ARNAQUE : double la perte
                    actual_loss = result.coins_lost_by_loser * 2;
                    insurance_msg = Some(format!(
                        "\u{1f480} L'assurance de <@{}> etait une **ARNAQUE** ! Double perte : **-{} coins** !",
                        loser_id, actual_loss
                    ));
                } else {
                    // Protection : reduit de 50%
                    actual_loss = result.coins_lost_by_loser / 2;
                    insurance_msg = Some(format!(
                        "\u{1f6e1}\u{fe0f} L'assurance a amorti le coup pour <@{}> ! Perte reduite : **-{} coins** (au lieu de {})",
                        loser_id, actual_loss, result.coins_lost_by_loser
                    ));
                }
            }

            // Gagnant recoit
            if let Err(e) = api
                .record_win(
                    &combat_record.guild_id,
                    winner_id,
                    result.coins_won,
                    result.stolen_bonus,
                )
                .await
            {
                error!(error = %e, "Erreur record_win");
            }
            // Perdant perd (montant ajuste par l'assurance)
            if let Err(e) = api
                .record_loss(&combat_record.guild_id, loser_id, actual_loss)
                .await
            {
                error!(error = %e, "Erreur record_loss");
            }

            // Primes : si le perdant a des primes, le gagnant les recupere
            let prime_amount = api
                .claim_primes(
                    &combat_record.guild_id,
                    loser_id,
                    winner_id,
                    if *winner_id == combat_record.attacker_id {
                        &combat_record.attacker_name
                    } else {
                        &combat_record.defender_name
                    },
                )
                .await
                .unwrap_or(0);

            if prime_amount > 0 {
                if let Err(e) = api
                    .record_coins_earned(&combat_record.guild_id, winner_id, prime_amount)
                    .await
                {
                    tracing::warn!(error = %e, "Echec API record_coins_earned");
                }
            }

            // Chaos event tracking
            if result.chaos_events_count > 0 {
                if let Err(e) = api
                    .increment_chaos_events(&combat_record.guild_id, &combat_record.attacker_id)
                    .await
                {
                    tracing::warn!(error = %e, "Echec API increment_chaos_events attacker");
                }
                if let Err(e) = api
                    .increment_chaos_events(&combat_record.guild_id, &combat_record.defender_id)
                    .await
                {
                    tracing::warn!(error = %e, "Echec API increment_chaos_events defender");
                }
            }

            // XP gains
            let level_gap = (attacker.level - defender.level).abs();
            let winner_is_underdog = level_gap >= 3 && result.is_giant_killer;
            let winner_xp_base = if winner_is_underdog { 30 } else { 15 };
            let loser_xp = 5i64;

            // Giant killer XP x2
            let winner_xp = if winner_is_underdog {
                winner_xp_base * 2
            } else {
                winner_xp_base
            };

            let mut xp_msg_lines: Vec<String> = Vec::new();

            if let Ok((_new_xp, new_level, leveled_up, stat_points)) =
                api.add_xp(&combat_record.guild_id, winner_id, winner_xp).await
            {
                xp_msg_lines.push(format!(
                    "\u{2b06}\u{fe0f} <@{}> gagne **+{} XP**{}",
                    winner_id,
                    winner_xp,
                    if winner_is_underdog { " (Giant Killer x2 !)" } else { "" }
                ));
                if leveled_up {
                    let title = progression::title_for_level(new_level);
                    xp_msg_lines.push(format!(
                        "\u{1f31f} **LEVEL UP !** <@{}> passe niveau **{}** \u{300c}{}\u{300d} ! (+{} points de stats)",
                        winner_id, new_level, title, stat_points
                    ));
                }
            }

            if let Ok((_new_xp, new_level, leveled_up, stat_points)) =
                api.add_xp(&combat_record.guild_id, loser_id, loser_xp).await
            {
                xp_msg_lines.push(format!(
                    "\u{2b06}\u{fe0f} <@{}> gagne **+{} XP**",
                    loser_id, loser_xp
                ));
                if leveled_up {
                    let title = progression::title_for_level(new_level);
                    xp_msg_lines.push(format!(
                        "\u{1f31f} **LEVEL UP !** <@{}> passe niveau **{}** \u{300c}{}\u{300d} ! (+{} points de stats)",
                        loser_id, new_level, title, stat_points
                    ));
                }
            }

            // Resoudre les paris (parieurs + bonus combattants)
            let (bet_results, fighter_bonus) = api
                .resolve_bets(&combat_record.id, Some(winner_id))
                .await
                .unwrap_or((vec![], None));

            let bet_msg = if !bet_results.is_empty() {
                let mut lines = vec![String::from("\u{1f3b2} **Resultats des paris :**")];
                for br in &bet_results {
                    if br.won {
                        lines.push(format!(
                            "\u{2705} **{}** gagne **{} coins** !",
                            br.bettor_name, br.payout
                        ));
                    } else {
                        lines.push(format!(
                            "\u{274c} **{}** perd sa mise de **{} coins**",
                            br.bettor_name, br.amount_bet
                        ));
                    }
                }
                // Bonus combattants
                if let Some(ref bonus) = fighter_bonus {
                    lines.push(String::new());
                    lines.push(format!(
                        "\u{1f4b0} **Pot des paris : {} coins**",
                        bonus.total_pot
                    ));
                    lines.push(format!(
                        "\u{1f451} <@{}> recoit **+{} coins** (10% du pot)",
                        winner_id, bonus.winner_bonus
                    ));
                    let loser_display = if *winner_id == combat_record.attacker_id {
                        &combat_record.defender_id
                    } else {
                        &combat_record.attacker_id
                    };
                    lines.push(format!(
                        "\u{1f3c5} <@{}> recoit **+{} coins** (5% du pot, merci d'avoir participe)",
                        loser_display, bonus.loser_bonus
                    ));
                }
                Some(lines.join("\n"))
            } else {
                None
            };

            let color = if result.chaos_events_count > 0 {
                0x9B59B6
            } else {
                0x57F287
            };

            let mut embed = CreateEmbed::new()
                .title("\u{2694}\u{fe0f} Resultat du Coup de Coude !")
                .description(&result.message)
                .color(color)
                .field(
                    "Combat",
                    format!(
                        "{} rounds | <@{}> : {}/{} HP | <@{}> : {}/{} HP",
                        result.total_rounds,
                        combat_record.attacker_id,
                        result.attacker_hp_final, result.attacker_hp_max,
                        combat_record.defender_id,
                        result.defender_hp_final, result.defender_hp_max,
                    ),
                    false,
                )
                .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
                .timestamp(serenity::model::Timestamp::now());

            if !xp_msg_lines.is_empty() {
                embed = embed.field(
                    "\u{1f4ca} Experience",
                    xp_msg_lines.join("\n"),
                    false,
                );
            }

            if prime_amount > 0 {
                embed = embed.field(
                    "\u{1f4b0} Primes recuperees !",
                    format!("<@{}> empoche {} coins de primes !", winner_id, prime_amount),
                    false,
                );
            }

            if let Some(ref ins_msg) = insurance_msg {
                embed = embed.field("\u{1f6e1}\u{fe0f} Assurance", ins_msg, false);
            }

            if let Some(ref bm) = bet_msg {
                embed = embed.field("\u{1f3b2} Paris", bm, false);
            }

            Some(embed)
        }
        _ => {
            // Match nul (accident_debile ou egalite)
            let had_accident = result.rounds.iter().any(|r| r.chaos_event == Some(ChaosEvent::AccidentDebile));
            if had_accident {
                // Les deux perdent la mise
                if let Err(e) = api
                    .record_draw(
                        &combat_record.guild_id,
                        &combat_record.attacker_id,
                        combat_record.mise,
                    )
                    .await
                {
                    tracing::warn!(error = %e, "Echec API record_draw attacker");
                }
                if let Err(e) = api
                    .record_draw(
                        &combat_record.guild_id,
                        &combat_record.defender_id,
                        combat_record.mise,
                    )
                    .await
                {
                    tracing::warn!(error = %e, "Echec API record_draw defender");
                }
                if let Err(e) = api
                    .increment_chaos_events(&combat_record.guild_id, &combat_record.attacker_id)
                    .await
                {
                    tracing::warn!(error = %e, "Echec API increment_chaos_events attacker");
                }
                if let Err(e) = api
                    .increment_chaos_events(&combat_record.guild_id, &combat_record.defender_id)
                    .await
                {
                    tracing::warn!(error = %e, "Echec API increment_chaos_events defender");
                }
            }

            // Resoudre les paris (egalite/accident = tout le monde perd)
            let (bet_results, _) = api
                .resolve_bets(&combat_record.id, None)
                .await
                .unwrap_or((vec![], None));

            let bet_msg = if !bet_results.is_empty() {
                let mut lines = vec![String::from("\u{1f3b2} **Resultats des paris :**")];
                for br in &bet_results {
                    lines.push(format!(
                        "\u{274c} **{}** perd sa mise de **{} coins**",
                        br.bettor_name, br.amount_bet
                    ));
                }
                Some(lines.join("\n"))
            } else {
                None
            };

            let mut embed = CreateEmbed::new()
                .title("\u{2694}\u{fe0f} Resultat du Coup de Coude !")
                .description(&result.message)
                .color(0x9B59B6)
                .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
                .timestamp(serenity::model::Timestamp::now());

            if let Some(ref bm) = bet_msg {
                embed = embed.field("\u{1f3b2} Paris", bm, false);
            }

            Some(embed)
        }
    }
}

async fn reply_ephemeral(ctx: &Context, component: &ComponentInteraction, content: &str) {
    if let Err(e) = component
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
        tracing::warn!(error = %e, "Echec response Discord");
    }
}
