use serenity::all::{
    ComponentInteraction, Context, CreateActionRow, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption, EditInteractionResponse,
};

use crate::shared::discord_helpers::component_reply_ephemeral as reply_ephemeral;

use crate::modules::coude::catalog::CatalogCacheKey;
use crate::modules::coude::GameApiKey;

/// Apres `Defer`, l'interaction est deja acquittee : on ne peut plus
/// `create_response`. On edite la reponse defer pour afficher l'erreur.
async fn edit_response_text(ctx: &Context, component: &ComponentInteraction, content: &str) {
    if let Err(e) = component
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(content).components(vec![]),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec edit_response Discord");
    }
}

pub const DEFEND_PREFIX: &str = "coude_defend:";
pub const DEFEND_SELECT_PREFIX: &str = "coude_defend_select:";

/// Items utilisables en defense.
const DEFENSIVE_ITEMS: &[&str] = &[
    "rage",         // +50% attaque -30% defense (risque)
    "double_coup",  // Lance le de deux fois par round
    "explosion",    // Les deux perdent 50% de la mise (defenseur uniquement)
    "mindgame",     // Revele la classe et HP adverses
    "bouclier",     // +20% DEF pendant le combat
    "antidote",     // Immunise contre le poison
    "poison",       // L'adversaire perd 5 HP par round
    "coup_traitre", // Reduit la DEF adverse de 50%
];

/// Gere le clic sur le bouton "Objet" — affiche l'inventaire du defenseur.
pub async fn handle_defend_button(ctx: &Context, component: &ComponentInteraction) {
    let combat_id_str = match component.data.custom_id.strip_prefix(DEFEND_PREFIX) {
        Some(id) => id,
        None => return,
    };

    // Capture le message_id du defi pour pouvoir l'editer apres resolution.
    let challenge_message_id = component.message.id.to_string();

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();
    let catalog = data.get::<CatalogCacheKey>().unwrap().clone();

    // Verifier le combat
    let combat_record = match api.get_combat(combat_id_str).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            reply_ephemeral(ctx, component, "Combat introuvable.").await;
            return;
        }
        Err(e) => {
            reply_ephemeral(ctx, component, &e).await;
            return;
        }
    };

    // Garde cross-guild : le combat doit appartenir a la guild d'ou vient le clic.
    if let Some(gid) = component.guild_id {
        if gid.to_string() != combat_record.guild_id {
            reply_ephemeral(ctx, component, "Ce combat n'appartient pas a cette guild.").await;
            return;
        }
    }

    // Verifier que c'est le defenseur
    if component.user.id.to_string() != combat_record.defender_id {
        reply_ephemeral(ctx, component, "Seul le defenseur peut utiliser un objet !").await;
        return;
    }

    if combat_record.status != "pending" {
        reply_ephemeral(ctx, component, "Ce combat n'est plus en attente.").await;
        return;
    }

    // Recuperer l'inventaire du defenseur
    let inventory = api
        .get_inventory(&combat_record.guild_id, &combat_record.defender_id)
        .await
        .unwrap_or_default();

    // Filtrer les items defensifs disponibles
    let mut options: Vec<CreateSelectMenuOption> = Vec::new();
    for item_key in DEFENSIVE_ITEMS {
        let has = inventory.iter().any(|i| i.item_key == *item_key && i.quantity > 0);
        if has {
            if let Some(shop_item) = catalog.get_item(item_key) {
                let qty = inventory
                    .iter()
                    .find(|i| i.item_key == *item_key)
                    .map(|i| i.quantity)
                    .unwrap_or(0);
                options.push(
                    CreateSelectMenuOption::new(
                        format!("{} {} (x{})", shop_item.emoji, shop_item.name, qty),
                        *item_key,
                    )
                    .description(shop_item.description.clone()),
                );
            }
        }
    }

    if options.is_empty() {
        reply_ephemeral(
            ctx,
            component,
            "Tu n'as aucun objet utilisable en defense ! Achete-en avec `/shop`.",
        )
        .await;
        return;
    }

    // Ajouter l'option "Accepter sans objet"
    options.insert(
        0,
        CreateSelectMenuOption::new("\u{270a} Accepter sans objet", "none")
            .description("Affronter a mains nues"),
    );

    // custom_id format : coude_defend_select:{combat_id}|{challenge_message_id}
    let select = CreateSelectMenu::new(
        format!("{}{}|{}", DEFEND_SELECT_PREFIX, combat_id_str, challenge_message_id),
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Choisis un objet pour te defendre...");

    let row = CreateActionRow::SelectMenu(select);

    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("\u{1f6e1}\u{fe0f} **Choisis un objet de defense :**")
                    .components(vec![row])
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }
}

/// Gere la selection d'un objet defensif → accepte le combat avec l'objet.
pub async fn handle_defend_select(ctx: &Context, component: &ComponentInteraction) {
    // Defer immediate : le handler enchaine 5 RPC (get_combat, use_item,
    // set_defender_special, delete_response, resolve_combat_internal) qui
    // depassent facilement le timeout Discord de 3s. On acknowledge
    // immediatement puis on delete la reponse a la fin.
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec defer defend_select");
    }

    let rest = match component.data.custom_id.strip_prefix(DEFEND_SELECT_PREFIX) {
        Some(id) => id,
        None => return,
    };
    // Parse "combat_id|challenge_message_id" — fallback retrocompat si seul combat_id.
    let (combat_id_str, challenge_message_id_opt): (&str, Option<&str>) = match rest.split_once('|') {
        Some((cid, mid)) => (cid, Some(mid)),
        None => (rest, None),
    };

    let selected_item = match &component.data.kind {
        serenity::all::ComponentInteractionDataKind::StringSelect { values } => {
            values.first().cloned().unwrap_or_default()
        }
        _ => return,
    };

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let combat_record = match api.get_combat(combat_id_str).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            edit_response_text(ctx, component, "Combat introuvable.").await;
            return;
        }
        Err(e) => {
            edit_response_text(ctx, component, &e).await;
            return;
        }
    };

    // Garde cross-guild
    if let Some(gid) = component.guild_id {
        if gid.to_string() != combat_record.guild_id {
            edit_response_text(ctx, component, "Ce combat n'appartient pas a cette guild.").await;
            return;
        }
    }

    if component.user.id.to_string() != combat_record.defender_id {
        edit_response_text(ctx, component, "Seul le defenseur peut faire ca !").await;
        return;
    }

    if combat_record.status != "pending" {
        edit_response_text(ctx, component, "Ce combat n'est plus en attente.").await;
        return;
    }

    // Consommer l'objet si ce n'est pas "none"
    if selected_item != "none" {
        if let Err(e) = api
            .use_item(
                &combat_record.guild_id,
                &combat_record.defender_id,
                &selected_item,
            )
            .await
        {
            edit_response_text(ctx, component, &format!("Erreur : {e}")).await;
            return;
        }

        // Enregistrer l'objet defensif dans le combat
        if let Err(e) = api.set_defender_special(combat_id_str, &selected_item).await {
            tracing::warn!(error = %e, "Erreur set_defender_special");
        }
    }

    drop(data);

    // Supprimer le select menu ephemeral
    if let Err(e) = component.delete_response(&ctx.http).await {
        tracing::warn!(error = %e, "Echec delete_response Discord");
    }

    // Resoudre le combat (meme logique que accepter)
    let result_embed =
        super::accepter::resolve_combat_internal(ctx, &combat_record, component.channel_id).await;

    if let Some(embed) = result_embed {
        // Editer le message de defi original pour afficher le resultat et
        // retirer les boutons : empeche le defenseur de recliquer "Accepter"
        // apres la resolution instantanee par item.
        let mut edited = false;
        if let Some(mid_str) = challenge_message_id_opt {
            if let Ok(mid) = mid_str.parse::<u64>() {
                let msg_id = serenity::model::id::MessageId::new(mid);
                match component
                    .channel_id
                    .edit_message(
                        &ctx.http,
                        msg_id,
                        serenity::all::EditMessage::new()
                            .embed(embed.clone())
                            .components(vec![]),
                    )
                    .await
                {
                    Ok(_) => edited = true,
                    Err(e) => tracing::warn!(error = %e, "Echec edit message defi original"),
                }
            }
        }

        // Fallback : si l'edit a echoue ou si on n'a pas le message_id
        // (compat legacy), on poste le resultat comme un nouveau message.
        if !edited {
            if let Err(e) = component
                .channel_id
                .send_message(
                    &ctx.http,
                    serenity::all::CreateMessage::new().embed(embed),
                )
                .await
            {
                tracing::warn!(error = %e, "Echec send_message resultat combat");
            }
        }
    }
}

