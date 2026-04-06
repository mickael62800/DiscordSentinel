use serenity::all::{
    ComponentInteraction, Context, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use serenity::model::id::ChannelId;
use tracing::error;
use uuid::Uuid;

use crate::db::Combat;
use crate::game::chaos::ChaosEvent;
use crate::game::combat;
use crate::game::progression;
use crate::handler::{GameDbKey, load_guild_config};

pub const ACCEPT_PREFIX: &str = "coude_accept:";

pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let combat_id_str = match component.data.custom_id.strip_prefix(ACCEPT_PREFIX) {
        Some(id) => id,
        None => return,
    };

    let combat_id = match Uuid::parse_str(combat_id_str) {
        Ok(id) => id,
        Err(_) => {
            reply_ephemeral(ctx, component, "ID de combat invalide.").await;
            return;
        }
    };

    let data = ctx.data.read().await;
    let db = data.get::<GameDbKey>().unwrap();

    let combat_record = match db.get_combat(combat_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            reply_ephemeral(ctx, component, "Combat introuvable.").await;
            return;
        }
        Err(e) => {
            reply_ephemeral(ctx, component, &format!("Erreur DB : {e}")).await;
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

    // Verifier l'expiration (24 heures)
    let elapsed = chrono::Utc::now()
        .signed_duration_since(combat_record.created_at)
        .num_seconds();
    if elapsed > 86400 {
        let _ = db.expire_combat(combat_id).await;
        reply_ephemeral(ctx, component, "Ce defi a expire ! (24h)").await;
        return;
    }

    // Charger la config pour le delai
    drop(data);
    let config = load_guild_config(ctx, &combat_record.guild_id).await;
    let delay_min = config.bet_delay_secs() / 60;

    // Passer le combat en phase "betting" avec le message_id
    let data = ctx.data.read().await;
    let db = data.get::<GameDbKey>().unwrap();
    let message_id = component.message.id.to_string();

    if let Err(e) = db.set_combat_betting(combat_id, &message_id).await {
        reply_ephemeral(ctx, component, &format!("Erreur DB : {e}")).await;
        return;
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

    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(waiting_embed)
                    .components(vec![]),
            ),
        )
        .await
        .ok();
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
    let db = data.get::<GameDbKey>().unwrap();

    let attacker = match db
        .get_player(&combat_record.guild_id, &combat_record.attacker_id)
        .await
    {
        Ok(Some(p)) => p,
        _ => return None,
    };

    let defender = match db
        .get_player(&combat_record.guild_id, &combat_record.defender_id)
        .await
    {
        Ok(Some(p)) => p,
        _ => return None,
    };

    let events = db
        .get_active_events(&combat_record.guild_id)
        .await
        .unwrap_or_default();

    let result = combat::resolve_combat(
        &attacker,
        &defender,
        combat_record.mise,
        combat_record.special_attack.as_deref(),
        combat_record.defender_special.as_deref(),
        &events,
    );

    // Mettre a jour la base de donnees
    let chaos_key = result.chaos_event.as_ref().map(|c| c.key());

    if let Err(e) = db
        .resolve_combat(
            combat_record.id,
            "accepted",
            result.winner_id.as_deref(),
            Some(result.attacker_roll),
            Some(result.defender_roll),
            chaos_key,
            &result.message,
            result.coins_won,
        )
        .await
    {
        error!(error = %e, "Erreur resolution combat DB");
        return None;
    }

    // Gestion des gains/pertes selon le resultat
    match (&result.winner_id, &result.loser_id) {
        (Some(winner_id), Some(loser_id)) => {
            // Verifier si le perdant a une assurance active
            let mut actual_loss = result.coins_lost_by_loser;
            let mut insurance_msg: Option<String> = None;

            if let Ok(Some(insurance)) = db
                .get_active_insurance(&combat_record.guild_id, loser_id)
                .await
            {
                // Consommer l'assurance
                let _ = db.expire_insurance(insurance.id).await;

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
            if let Err(e) = db
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
            if let Err(e) = db
                .record_loss(&combat_record.guild_id, loser_id, actual_loss)
                .await
            {
                error!(error = %e, "Erreur record_loss");
            }

            // Primes : si le perdant a des primes, le gagnant les recupere
            let prime_amount = db
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
                let _ = db
                    .record_coins_earned(&combat_record.guild_id, winner_id, prime_amount)
                    .await;
            }

            // Chaos event tracking
            if result.chaos_event.is_some() {
                let _ = db
                    .increment_chaos_events(&combat_record.guild_id, &combat_record.attacker_id)
                    .await;
                let _ = db
                    .increment_chaos_events(&combat_record.guild_id, &combat_record.defender_id)
                    .await;
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
                db.add_xp(&combat_record.guild_id, winner_id, winner_xp).await
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
                db.add_xp(&combat_record.guild_id, loser_id, loser_xp).await
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

            // Inversion special
            if combat_record.special_attack.as_deref() == Some("inversion") {
                let atk_coins = attacker.coins;
                let def_coins = defender.coins;
                let _ = db
                    .set_player_coins(&combat_record.guild_id, &attacker.user_id, def_coins)
                    .await;
                let _ = db
                    .set_player_coins(&combat_record.guild_id, &defender.user_id, atk_coins)
                    .await;
            }

            // Resoudre les paris (parieurs + bonus combattants)
            let (bet_results, fighter_bonus) = db
                .resolve_bets(combat_record.id, Some(winner_id))
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
                            br.bettor_name, br.amount
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

            let color = if result.chaos_event.is_some() {
                0x9B59B6
            } else {
                0x57F287
            };

            let mut embed = CreateEmbed::new()
                .title("\u{2694}\u{fe0f} Resultat du Coup de Coude !")
                .description(&result.message)
                .color(color)
                .field(
                    "Degats",
                    format!(
                        "<@{}> : {} dmg (roll {}) | <@{}> : {} dmg (roll {})",
                        combat_record.attacker_id,
                        result.attacker_damage,
                        result.attacker_roll,
                        combat_record.defender_id,
                        result.defender_damage,
                        result.defender_roll
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
            if let Some(ChaosEvent::AccidentDebile) = result.chaos_event {
                // Les deux perdent la mise
                let _ = db
                    .record_draw(
                        &combat_record.guild_id,
                        &combat_record.attacker_id,
                        combat_record.mise,
                    )
                    .await;
                let _ = db
                    .record_draw(
                        &combat_record.guild_id,
                        &combat_record.defender_id,
                        combat_record.mise,
                    )
                    .await;
                let _ = db
                    .increment_chaos_events(&combat_record.guild_id, &combat_record.attacker_id)
                    .await;
                let _ = db
                    .increment_chaos_events(&combat_record.guild_id, &combat_record.defender_id)
                    .await;
            }

            // Resoudre les paris (egalite/accident = tout le monde perd)
            let (bet_results, _) = db
                .resolve_bets(combat_record.id, None)
                .await
                .unwrap_or((vec![], None));

            let bet_msg = if !bet_results.is_empty() {
                let mut lines = vec![String::from("\u{1f3b2} **Resultats des paris :**")];
                for br in &bet_results {
                    lines.push(format!(
                        "\u{274c} **{}** perd sa mise de **{} coins**",
                        br.bettor_name, br.amount
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
