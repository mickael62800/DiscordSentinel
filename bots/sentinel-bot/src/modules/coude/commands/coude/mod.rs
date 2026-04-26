//! Slash command `/coude` — défier un joueur en Coup de Coude.
//!
//! Le fichier `mod.rs` ne contient que la registration et l'orchestration
//! du handler (parsing options → validations → création combat → dispatch UI).
//! Les constructions d'embeds et boutons vivent dans `challenge_ui`.

mod challenge_ui;

use serenity::all::{
    ButtonStyle, CommandDataOptionValue, CommandInteraction, CommandOptionType, ComponentInteraction,
    Context, CreateActionRow, CreateButton, CreateCommand, CreateCommandOption, CreateEmbed,
    CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    EditInteractionResponse, UserId,
};

use sentinel_shared::discord_helpers::reply_ephemeral;

use crate::modules::coude::catalog::CatalogCacheKey;
use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

use challenge_ui::{
    build_bloodbath_embed, build_challenge_buttons, build_challenge_embed, build_handicap_warning,
    build_notification_embed, build_surprise_embed,
};

/// Prefixe du bouton "Confirmer" affiche avant la creation du combat (pour
/// que l'attaquant voit ses PV avant de lancer le defi).
pub const PRECONFIRM_OK_PREFIX: &str = "coude_prec_ok|";
/// Prefixe du bouton "Annuler" du meme flow.
pub const PRECONFIRM_NO_PREFIX: &str = "coude_prec_no|";

pub fn register() -> CreateCommand {
    CreateCommand::new("coude")
        .description("Defie un joueur en Coup de Coude !")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Le joueur a defier")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "mise",
                "Montant de la mise (defaut: 10)",
            )
            .required(false)
            .min_int_value(1),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "special", "Attaque speciale (item)")
                .required(false)
                .add_string_choice("Attaque surprise", "surprise")
                .add_string_choice("Double coup", "double_coup")
                .add_string_choice("Coup traitre", "coup_traitre")
                .add_string_choice("Rage", "rage")
                .add_string_choice("Poison", "poison")
                .add_string_choice("Mindgame", "mindgame")
                .add_string_choice("Bouclier", "bouclier")
                .add_string_choice("Antidote", "antidote")
                .add_string_choice("Explosion", "explosion"),
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
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_combats()).await {
        return;
    }
    if !config.enabled() {
        reply_ephemeral(ctx, command, "Le jeu Coup de Coude est desactive sur ce serveur.").await;
        return;
    }

    let target_id = command
        .data
        .options
        .iter()
        .find(|o| o.name == "cible")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        })
        .unwrap();

    if target_id == command.user.id {
        reply_ephemeral(ctx, command, "Tu ne peux pas te defier toi-meme !").await;
        return;
    }

    let mise = command
        .data
        .options
        .iter()
        .find(|o| o.name == "mise")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(config.default_bet());

    let special = command
        .data
        .options
        .iter()
        .find(|o| o.name == "special")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        });

    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };

    if target.bot {
        reply_ephemeral(ctx, command, "Tu ne peux pas defier un bot !").await;
        return;
    }

    // Defer ephemeral : /coude enchaine jusqu'a 5 appels API avant de
    // montrer le preconfirm. Sans defer, Discord coupait l'interaction.
    if !crate::modules::coude::interaction_helper::defer_ephemeral(ctx, command).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let catalog = data.get::<CatalogCacheKey>().unwrap().clone();

    // Creer/recuperer les joueurs
    let attacker = match api
        .get_or_create_player(&guild_id, &command.user.id.to_string(), &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let defender_player = match api
        .get_or_create_player(&guild_id, &target.id.to_string(), &target.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    // HP minimum pour combattre (10 % par defaut).
    // On bloque avant meme d'ouvrir la preconfirmation : inutile de faire
    // cliquer l'attaquant pour ensuite lui dire qu'il est en KO.
    let hp_current_now = attacker.hp_current.unwrap_or(100);
    let hp_max_now = attacker.hp_max.unwrap_or(100);
    let hp_pct_now = if hp_max_now > 0 { (hp_current_now * 100) / hp_max_now } else { 0 };
    if hp_pct_now < 10 {
        crate::modules::coude::interaction_helper::followup_text(
            ctx,
            command,
            &format!(
                "\u{1f480} Tu es trop bas en PV pour combattre ! ({}/{} — {}%)\n\
                 Utilise `/repos` (cooldown 12h) ou attends la regen passive.",
                hp_current_now, hp_max_now, hp_pct_now
            ),
        )
        .await;
        return;
    }

    // Matchmaking check (handicap sera recalcule apres la confirmation)
    let level_gap = (attacker.level - defender_player.level).abs();
    let (_handicap, blocked) =
        catalog.matchmaking_handicap(attacker.level, defender_player.level);

    if blocked {
        crate::modules::coude::interaction_helper::followup_text(
            ctx,
            command,
            &format!(
                "Ecart de niveau trop important ! ({} niveaux d'ecart, max 9)\n\
                 Ton niveau : {} | Niveau de <@{}> : {}",
                level_gap, attacker.level, target.id, defender_player.level
            ),
        )
        .await;
        return;
    }

    // Verifier la mise (limites depuis la config)
    if mise < config.min_bet() {
        crate::modules::coude::interaction_helper::followup_text(
            ctx,
            command,
            &format!("La mise minimum est de {} coins.", config.min_bet()),
        )
        .await;
        return;
    }
    if mise > config.max_bet() {
        crate::modules::coude::interaction_helper::followup_text(
            ctx,
            command,
            &format!("La mise maximum est de {} coins.", config.max_bet()),
        )
        .await;
        return;
    }
    if attacker.coins < mise {
        crate::modules::coude::interaction_helper::followup_text(
            ctx,
            command,
            &format!(
                "Tu n'as pas assez de coins ! (tu as {} coins, mise demandee : {})",
                attacker.coins, mise
            ),
        )
        .await;
        return;
    }

    // Verifier pas de combat en cours
    if let Ok(Some(_)) = api
        .get_pending_combat_for_attacker(&guild_id, &command.user.id.to_string())
        .await
    {
        crate::modules::coude::interaction_helper::followup_text(ctx, command, "Tu as deja un defi en attente !").await;
        return;
    }

    // Verifier l'item special (has_item seulement — la consommation est
    // differee apres la confirmation pour ne pas gaspiller l'item si
    // l'attaquant annule le preconfirm).
    if let Some(ref item_key) = special {
        let has = match api
            .has_item(&guild_id, &command.user.id.to_string(), item_key)
            .await
        {
            Ok(h) => h,
            Err(e) => {
                crate::modules::coude::interaction_helper::followup_text(ctx, command, &format!("Erreur API : {e}")).await;
                return;
            }
        };
        if !has {
            crate::modules::coude::interaction_helper::followup_text(
                ctx,
                command,
                &format!("Tu n'as pas l'objet **{}** dans ton inventaire !", item_key),
            )
            .await;
            return;
        }
    }

    // Preconfirmation : on montre les PV de l'attaquant + boutons oui/non.
    // Le combat n'est PAS encore cree a ce stade ; ca evite :
    // - de consommer l'item si l'attaquant change d'avis ;
    // - de laisser une row pending en DB si l'attaquant abandonne ;
    // - de surprendre quelqu'un qui ne savait pas qu'il etait low HP.
    let hp_current = attacker.hp_current.unwrap_or(100);
    let hp_max = attacker.hp_max.unwrap_or(100);
    let hp_pct = if hp_max > 0 { (hp_current * 100) / hp_max } else { 0 };
    let hp_warn = if hp_pct <= 25 {
        "\n\n\u{26a0}\u{fe0f} **Tu es tres bas en PV !** Si tu lances ce combat tu risques une faillite HP rapide."
    } else if hp_pct <= 50 {
        "\n\n\u{26a0}\u{fe0f} Tu as moins de la moitie de tes PV. Pense a `/repos` avant si tu peux."
    } else {
        ""
    };

    // Blocage : impossible de defier un joueur a 0 coin (il n'a rien a perdre,
    // et l'attaquant ne peut rien gagner — duel deséquilibré).
    // Avertissement : si le defenseur a moins que la mise, on previent que le
    // gain sera cap sur son solde reel.
    // Refetch defender pour avoir ses coins a jour (il a pu changer
    // entre le premier fetch et maintenant). On reutilise defender_player
    // pour les checks plus simples si besoin de perf, mais ici la row
    // coins peut avoir bouge.
    let defender_coins_warn = match api
        .get_or_create_player(&guild_id, &target.id.to_string(), &target.name)
        .await
    {
        Ok(def) if def.coins <= 0 => {
            crate::modules::coude::interaction_helper::followup_text(
                ctx,
                command,
                &format!(
                    "Impossible de defier <@{}> : ce joueur n'a aucun coin. Pas de duel sans enjeu !",
                    target.id
                ),
            )
            .await;
            return;
        }
        Ok(def) if def.coins < mise => format!(
            "\n\n\u{26a0}\u{fe0f} **<@{}> n'a que {} coins !** Si tu gagnes, tu ne recupereras que **{} coins** (pas {}). Si tu perds, tu perdras bien les {} coins mises.",
            target.id, def.coins, def.coins, mise, mise
        ),
        _ => String::new(),
    };
    let hp_warn = format!("{}{}", hp_warn, defender_coins_warn);

    let special_suffix = special
        .as_deref()
        .map(|s| format!(" | Special : **{}**", s))
        .unwrap_or_default();
    let special_for_id = special.as_deref().unwrap_or("-");

    let custom_ok = format!(
        "{}{}|{}|{}",
        PRECONFIRM_OK_PREFIX, target.id, mise, special_for_id
    );
    let custom_no = format!(
        "{}{}|{}|{}",
        PRECONFIRM_NO_PREFIX, target.id, mise, special_for_id
    );

    let confirm_embed = CreateEmbed::new()
        .title("\u{2694}\u{fe0f} Confirmer le defi ?")
        .description(format!(
            "Tu vas defier <@{}> pour **{} coins**.{}\n\n\
             \u{2764}\u{fe0f} **Tes PV actuels : {} / {}**{}\n\n\
             Lancer le combat ?",
            target.id, mise, special_suffix, hp_current, hp_max, hp_warn
        ))
        .color(if hp_pct <= 25 { 0xE74C3C } else { 0xF1C40F })
        .footer(CreateEmbedFooter::new(
            "Coup de Coude | Sentinel — cette confirmation t'est reservee",
        ))
        .timestamp(serenity::model::Timestamp::now());

    let row = CreateActionRow::Buttons(vec![
        CreateButton::new(custom_ok)
            .label("Confirmer")
            .style(ButtonStyle::Success)
            .emoji(serenity::model::channel::ReactionType::Unicode(
                "\u{2705}".to_string(),
            )),
        CreateButton::new(custom_no)
            .label("Annuler")
            .style(ButtonStyle::Secondary)
            .emoji(serenity::model::channel::ReactionType::Unicode(
                "\u{274c}".to_string(),
            )),
    ]);

    // Followup ephemeral avec embed + components (on a defer plus haut).
    if let Err(e) = command
        .create_followup(
            &ctx.http,
            serenity::all::CreateInteractionResponseFollowup::new()
                .embed(confirm_embed)
                .components(vec![row])
                .ephemeral(true),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec followup Discord preconfirm");
    }
}

/// Handler du bouton "Confirmer" affiche par `/coude` avant la creation du
/// combat. Parse le custom_id, rejoue les validations minimales, consomme
/// l'item eventuel puis cree le combat et poste le defi normal.
pub async fn handle_preconfirm_ok(ctx: &Context, component: &ComponentInteraction) {
    let (target_id_str, mise, special) = match parse_preconfirm_id(&component.data.custom_id, PRECONFIRM_OK_PREFIX) {
        Some(x) => x,
        None => {
            edit_component_message(ctx, component, "Custom id invalide.").await;
            return;
        }
    };

    let guild_id = match component.guild_id {
        Some(id) => id.to_string(),
        None => {
            edit_component_message(ctx, component, "Commande serveur uniquement.").await;
            return;
        }
    };

    let config = load_guild_config(ctx, &guild_id).await;

    let target_id = match target_id_str.parse::<u64>() {
        Ok(v) => UserId::new(v),
        Err(_) => {
            edit_component_message(ctx, component, "Cible invalide.").await;
            return;
        }
    };
    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            edit_component_message(ctx, component, "Cible introuvable.").await;
            return;
        }
    };

    let attacker_user = &component.user;

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let catalog = data.get::<CatalogCacheKey>().unwrap().clone();

    // Re-fetch attacker pour check coins a jour (un vol/combat a pu passer
    // entre le /coude et le clic sur Confirmer).
    let attacker = match api
        .get_or_create_player(&guild_id, &attacker_user.id.to_string(), &attacker_user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            edit_component_message(ctx, component, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    if attacker.coins < mise {
        edit_component_message(
            ctx,
            component,
            &format!(
                "Ton solde n'est plus suffisant ! (tu as {} coins, mise demandee : {})",
                attacker.coins, mise
            ),
        )
        .await;
        return;
    }

    // Revalidation HP : si l'attaquant a pris des degats entre le /coude et
    // le clic Confirmer, on refuse plutot que de lancer un combat perdu d'avance.
    let hp_c = attacker.hp_current.unwrap_or(100);
    let hp_m = attacker.hp_max.unwrap_or(100);
    let pct = if hp_m > 0 { (hp_c * 100) / hp_m } else { 0 };
    if pct < 10 {
        edit_component_message(
            ctx,
            component,
            &format!(
                "\u{1f480} Tu es trop bas en PV maintenant ({}/{} — {}%). Defi annule.",
                hp_c, hp_m, pct
            ),
        )
        .await;
        return;
    }

    // Pas de combat en cours entre temps
    if let Ok(Some(_)) = api
        .get_pending_combat_for_attacker(&guild_id, &attacker_user.id.to_string())
        .await
    {
        edit_component_message(ctx, component, "Tu as deja un defi en attente !").await;
        return;
    }

    let defender_player = match api
        .get_or_create_player(&guild_id, &target.id.to_string(), &target.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            edit_component_message(ctx, component, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let (handicap, _blocked) = catalog.matchmaking_handicap(attacker.level, defender_player.level);

    // Consommer l'item SEULEMENT ici (apres confirm).
    if special != "-" {
        let has = api
            .has_item(&guild_id, &attacker_user.id.to_string(), &special)
            .await
            .unwrap_or(false);
        if !has {
            edit_component_message(
                ctx,
                component,
                &format!("Tu n'as plus l'objet **{}** dans ton inventaire !", special),
            )
            .await;
            return;
        }
        if let Err(e) = api
            .use_item(&guild_id, &attacker_user.id.to_string(), &special)
            .await
        {
            edit_component_message(ctx, component, &format!("Erreur API : {e}")).await;
            return;
        }
    }

    let combat_channel = match config.channel_combats() {
        Some(c) => c,
        None => {
            edit_component_message(ctx, component, "Salon combats non configure.").await;
            return;
        }
    };

    let special_opt = if special == "-" { None } else { Some(special.as_str()) };

    let combat = match api
        .create_combat(
            &guild_id,
            Some(&combat_channel),
            &attacker_user.id.to_string(),
            &attacker_user.name,
            &target.id.to_string(),
            &target.name,
            mise,
            special_opt,
        )
        .await
    {
        Ok(c) => c,
        Err(e) => {
            edit_component_message(ctx, component, &format!("Erreur creation combat : {e}")).await;
            return;
        }
    };

    // Attaque surprise : auto-resolve direct.
    // Important : poster l'embed d'annonce PUIS l'embed de resultat retourne
    // par resolve_combat_internal. Avant, la valeur de retour etait ignoree
    // et le combat semblait ne pas avoir eu lieu (seule l'annonce s'affichait).
    drop(data);

    if special_opt == Some("surprise") {
        // On tente l'auto-resolve. Si l'API refuse parce que le defenseur
        // possede un item de contre (Explosion) + `surprise_allow_defender_counter`
        // active, on bascule sur le flow de defi normal pour laisser au
        // defenseur une chance de riposter.
        match super::accepter::resolve_combat_internal_ex(ctx, &combat, component.channel_id).await
        {
            super::accepter::ResolveOutcome::Resolved(embed) => {
                let _ = component
                    .channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new()
                            .embed(build_surprise_embed(attacker_user.id, target.id)),
                    )
                    .await;
                super::accepter::post_combat_embed_animated(
                    ctx,
                    component.channel_id,
                    embed,
                    combat.mise,
                )
                .await;
                edit_component_message(ctx, component, "\u{2705} Defi surprise resolu !").await;
                return;
            }
            super::accepter::ResolveOutcome::DefenderCanCounter => {
                // On ne poste pas l'embed "surprise auto-resolu" : on
                // tombe sur le flow normal en bas (challenge embed + boutons).
                tracing::info!(
                    combat_id = %combat.id,
                    "Surprise bloquee : defenseur possede item de contre, bascule sur flow normal"
                );
                // fall-through
            }
            super::accepter::ResolveOutcome::Failed => {
                tracing::error!(
                    combat_id = %combat.id,
                    "resolve_combat_internal a echoue pour un combat surprise"
                );
                edit_component_message(
                    ctx,
                    component,
                    "\u{26a0}\u{fe0f} Defi surprise lance mais la resolution a echoue (voir logs bot).",
                )
                .await;
                return;
            }
        }
    }

    // Re-acquerir data/api : ils ont pu etre drop dans la branche surprise
    // (fall-through apres DefenderCanCounter).
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // Bloodbath : auto-accept — meme correction que surprise.
    let events = api.get_active_events(&guild_id).await.unwrap_or_default();
    let bloodbath = events.iter().any(|e| e.event_type == "bloodbath");
    if bloodbath {
        drop(data);
        let _ = component
            .channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new().embed(build_bloodbath_embed(attacker_user.id, target.id)),
            )
            .await;
        match super::accepter::resolve_combat_internal(ctx, &combat, component.channel_id).await {
            Some(embed) => {
                super::accepter::post_combat_embed_animated(
                    ctx,
                    component.channel_id,
                    embed,
                    combat.mise,
                )
                .await;
                edit_component_message(ctx, component, "\u{2705} Defi Bloodbath resolu !").await;
            }
            None => {
                tracing::error!(
                    combat_id = %combat.id,
                    "resolve_combat_internal a retourne None pour un combat bloodbath"
                );
                edit_component_message(
                    ctx,
                    component,
                    "\u{26a0}\u{fe0f} Defi Bloodbath lance mais la resolution a echoue.",
                )
                .await;
            }
        }
        return;
    }

    // Flow normal : poster le defi public avec boutons
    let special_label = special_opt
        .map(|s| format!(" | Special : **{}**", s))
        .unwrap_or_default();
    let handicap_warning = build_handicap_warning(
        attacker_user.id,
        attacker.level,
        target.id,
        defender_player.level,
        handicap,
    );
    let challenge_embed = build_challenge_embed(
        attacker_user.id,
        target.id,
        mise,
        &special_label,
        &handicap_warning,
    );
    let challenge_row = build_challenge_buttons(&combat.id);

    if let Err(e) = component
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .embed(challenge_embed)
                .components(vec![challenge_row]),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec send_message defi");
    }

    // Notifier dans le salon notifications
    if let Some(notif_ch) = config.channel_notifications() {
        if let Ok(ch_id) = notif_ch.parse::<u64>() {
            let notif_embed = build_notification_embed(
                target.id,
                &attacker_user.name,
                mise,
                &combat_channel,
            );
            let _ = serenity::model::id::ChannelId::new(ch_id)
                .send_message(&ctx.http, CreateMessage::new().embed(notif_embed))
                .await;
        }
    }

    edit_component_message(ctx, component, "\u{2705} Defi envoye dans le salon combats !").await;
}

/// Handler du bouton "Annuler" du preconfirm.
pub async fn handle_preconfirm_no(ctx: &Context, component: &ComponentInteraction) {
    edit_component_message(ctx, component, "\u{274c} Defi annule avant envoi. Aucune mise prelevee.").await;
}

fn parse_preconfirm_id(id: &str, prefix: &str) -> Option<(String, i64, String)> {
    let rest = id.strip_prefix(prefix)?;
    let mut parts = rest.splitn(3, '|');
    let target = parts.next()?.to_string();
    let mise: i64 = parts.next()?.parse().ok()?;
    let special = parts.next()?.to_string();
    Some((target, mise, special))
}

async fn edit_component_message(ctx: &Context, component: &ComponentInteraction, content: &str) {
    // On accuse reception (defer update) puis on edit le message ephemere
    // pour virer les boutons et mettre le status final.
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .embeds(vec![])
                    .components(vec![]),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec update preconfirm message");
        // Fallback : follow-up ephemere
        let _ = component
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content(content).components(vec![]),
            )
            .await;
    }
}

