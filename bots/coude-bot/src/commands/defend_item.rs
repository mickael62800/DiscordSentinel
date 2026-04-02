use serenity::all::{
    ComponentInteraction, Context, CreateActionRow, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption,
};
use uuid::Uuid;

use crate::game::shop;
use crate::handler::GameDbKey;

pub const DEFEND_PREFIX: &str = "coude_defend:";
pub const DEFEND_SELECT_PREFIX: &str = "coude_defend_select:";

/// Items utilisables en defense.
const DEFENSIVE_ITEMS: &[&str] = &[
    "rage",         // +50 attaque -50 defense (risque)
    "double_coup",  // Lance le de deux fois
    "explosion",    // Les deux perdent
    "mindgame",     // Voir le roll adverse
    "inversion",    // Echange les coins
];

/// Gere le clic sur le bouton "Objet" — affiche l'inventaire du defenseur.
pub async fn handle_defend_button(ctx: &Context, component: &ComponentInteraction) {
    let combat_id_str = match component.data.custom_id.strip_prefix(DEFEND_PREFIX) {
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

    // Verifier le combat
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
    let inventory = db
        .get_inventory(&combat_record.guild_id, &combat_record.defender_id)
        .await
        .unwrap_or_default();

    // Filtrer les items defensifs disponibles
    let mut options: Vec<CreateSelectMenuOption> = Vec::new();
    for item_key in DEFENSIVE_ITEMS {
        let has = inventory.iter().any(|i| i.item_key == *item_key && i.quantity > 0);
        if has {
            if let Some(shop_item) = shop::get_item(item_key) {
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
                    .description(shop_item.description),
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

    let select = CreateSelectMenu::new(
        format!("{}{}",  DEFEND_SELECT_PREFIX, combat_id_str),
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Choisis un objet pour te defendre...");

    let row = CreateActionRow::SelectMenu(select);

    component
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
        .ok();
}

/// Gere la selection d'un objet defensif → accepte le combat avec l'objet.
pub async fn handle_defend_select(ctx: &Context, component: &ComponentInteraction) {
    let combat_id_str = match component.data.custom_id.strip_prefix(DEFEND_SELECT_PREFIX) {
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

    let selected_item = match &component.data.kind {
        serenity::all::ComponentInteractionDataKind::StringSelect { values } => {
            values.first().cloned().unwrap_or_default()
        }
        _ => return,
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

    if component.user.id.to_string() != combat_record.defender_id {
        reply_ephemeral(ctx, component, "Seul le defenseur peut faire ca !").await;
        return;
    }

    if combat_record.status != "pending" {
        reply_ephemeral(ctx, component, "Ce combat n'est plus en attente.").await;
        return;
    }

    // Consommer l'objet si ce n'est pas "none"
    if selected_item != "none" {
        if let Err(e) = db
            .use_item(
                &combat_record.guild_id,
                &combat_record.defender_id,
                &selected_item,
            )
            .await
        {
            reply_ephemeral(ctx, component, &format!("Erreur : {e}")).await;
            return;
        }

        // Enregistrer l'objet defensif dans le combat
        if let Err(e) = db.set_defender_special(combat_id, &selected_item).await {
            tracing::warn!(error = %e, "Erreur set_defender_special");
        }
    }

    drop(data);

    // Supprimer le select menu ephemeral
    let _ = component.delete_response(&ctx.http).await;

    // Resoudre le combat (meme logique que accepter)
    let result_embed =
        super::accepter::resolve_combat_internal(ctx, &combat_record, component.channel_id).await;

    if let Some(embed) = result_embed {
        // Poster le resultat dans le channel (pas en update car le select etait ephemeral)
        let _ = component
            .channel_id
            .send_message(
                &ctx.http,
                serenity::all::CreateMessage::new().embed(embed),
            )
            .await;

        // Supprimer le message original de defi (celui avec les boutons)
        if let Ok(msg_id) = combat_record.channel_id.parse::<u64>() {
            // On ne peut pas facilement retrouver le message original ici
            // Le message de defi sera obsolete mais les boutons ne fonctionneront plus (status != pending)
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
