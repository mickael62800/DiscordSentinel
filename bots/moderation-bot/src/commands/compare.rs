//! MOD #5 — Commande `/compare` : affiche un historique croise entre deux
//! utilisateurs pour permettre a un moderateur de comparer leur reputation
//! (ex: decider lequel est le troll dans un conflit).
//!
//! Appelle 2x `get_history` via l'API moderation (pas de nouvel endpoint).

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{error, warn};

use sentinel_shared::embeds::info_embed;

use crate::api_client::UserHistory;
use crate::handler::ModerationApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("compare")
        .description("Comparer l'historique de sanctions de deux utilisateurs")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user1", "Premier utilisateur")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user2", "Second utilisateur")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    // Defer : 2 appels API get_history peuvent depasser 3s cumules.
    if let Err(e) = command.create_response(
        &ctx.http,
        CreateInteractionResponse::Defer(
            CreateInteractionResponseMessage::new().ephemeral(true),
        ),
    ).await {
        warn!(error = %e, cmd = "compare", "Echec defer interaction Discord");
        return;
    }

    let user1_id = match command
        .data
        .options
        .iter()
        .find(|o| o.name == "user1")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        }) {
        Some(id) => id,
        None => { reply_text(ctx, command, "Parametre 'user1' manquant.").await; return; }
    };
    let user2_id = match command
        .data
        .options
        .iter()
        .find(|o| o.name == "user2")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        }) {
        Some(id) => id,
        None => { reply_text(ctx, command, "Parametre 'user2' manquant.").await; return; }
    };

    if user1_id == user2_id {
        reply_text(ctx, command, "Les deux utilisateurs doivent etre differents.").await;
        return;
    }

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            reply_text(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let user1 = user1_id.to_user(&ctx.http).await.ok();
    let user2 = user2_id.to_user(&ctx.http).await.ok();
    let name1 = user1.as_ref().map(|u| u.name.clone()).unwrap_or_else(|| "?".into());
    let name2 = user2.as_ref().map(|u| u.name.clone()).unwrap_or_else(|| "?".into());

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => {
            error!("ModerationApiKey manquant");
            return;
        }
    };

    // Appels paralleles via tokio::join
    let guild_str = guild_id.to_string();
    let u1_str = user1_id.to_string();
    let u2_str = user2_id.to_string();
    let (h1_res, h2_res) = tokio::join!(
        api.get_history(&guild_str, &u1_str),
        api.get_history(&guild_str, &u2_str),
    );

    let h1 = match h1_res {
        Ok(h) => h,
        Err(e) => {
            error!(error = %e, user = %name1, "Erreur recuperation historique user1");
            reply_text(ctx, command, &format!("Erreur historique {name1} : {e}")).await;
            return;
        }
    };
    let h2 = match h2_res {
        Ok(h) => h,
        Err(e) => {
            error!(error = %e, user = %name2, "Erreur recuperation historique user2");
            reply_text(ctx, command, &format!("Erreur historique {name2} : {e}")).await;
            return;
        }
    };

    let summary = build_comparison_line(&h1, &h2);

    let embed = info_embed(format!("\u{2696}\u{fe0f} Comparaison — @{name1} vs @{name2}"))
        .description(summary)
        .field(
            format!("\u{1f464} @{name1}"),
            format_history_block(&h1),
            true,
        )
        .field(
            format!("\u{1f464} @{name2}"),
            format_history_block(&h2),
            true,
        );

    if let Err(e) = command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().embed(embed),
        )
        .await
    {
        warn!(error = %e, "Failed to send compare response");
    }
}

/// Phrase d'analyse : qui a le plus de warns / mutes / bans.
fn build_comparison_line(h1: &UserHistory, h2: &UserHistory) -> String {
    let total1 = h1.total_warns + h1.total_mutes + h1.total_bans;
    let total2 = h2.total_warns + h2.total_mutes + h2.total_bans;

    let verdict = match total1.cmp(&total2) {
        std::cmp::Ordering::Greater => format!(
            "**@{}** a plus de sanctions au total ({} vs {})",
            h1.target_name, total1, total2
        ),
        std::cmp::Ordering::Less => format!(
            "**@{}** a plus de sanctions au total ({} vs {})",
            h2.target_name, total2, total1
        ),
        std::cmp::Ordering::Equal => {
            format!("Les deux utilisateurs ont le meme nombre de sanctions ({total1})")
        }
    };
    verdict
}

/// Bloc de comptage par type pour un user.
fn format_history_block(h: &UserHistory) -> String {
    format!(
        "\u{26a0}\u{fe0f} Warns : **{}**\n\u{1f507} Mutes : **{}**\n\u{1f6ab} Bans : **{}**\n\u{1f4ca} Total : **{}**",
        h.total_warns,
        h.total_mutes,
        h.total_bans,
        h.total_warns + h.total_mutes + h.total_bans
    )
}

async fn reply_text(ctx: &Context, command: &CommandInteraction, content: &str) {
    // Apres Defer, on edit la reponse (create_response ferait Unknown Interaction).
    if let Err(e) = command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().content(content),
        )
        .await
    {
        warn!(error = %e, "Failed to send compare error");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_client::ModerationActionResponse;

    fn mk_history(name: &str, warns: u32, mutes: u32, bans: u32) -> UserHistory {
        UserHistory {
            target_id: "42".into(),
            target_name: name.into(),
            total_warns: warns,
            total_mutes: mutes,
            total_bans: bans,
            actions: Vec::<ModerationActionResponse>::new(),
        }
    }

    #[test]
    fn compare_h1_more() {
        let h1 = mk_history("alice", 5, 2, 1);
        let h2 = mk_history("bob", 1, 0, 0);
        let line = build_comparison_line(&h1, &h2);
        assert!(line.contains("@alice"));
        assert!(line.contains("(8 vs 1)"));
    }

    #[test]
    fn compare_h2_more() {
        let h1 = mk_history("alice", 0, 0, 0);
        let h2 = mk_history("bob", 3, 1, 1);
        let line = build_comparison_line(&h1, &h2);
        assert!(line.contains("@bob"));
        assert!(line.contains("(5 vs 0)"));
    }

    #[test]
    fn compare_equal() {
        let h1 = mk_history("alice", 2, 1, 0);
        let h2 = mk_history("bob", 1, 2, 0);
        let line = build_comparison_line(&h1, &h2);
        assert!(line.contains("meme nombre"));
    }

    #[test]
    fn format_block_values() {
        let h = mk_history("x", 3, 2, 1);
        let b = format_history_block(&h);
        assert!(b.contains("**3**"));
        assert!(b.contains("**2**"));
        assert!(b.contains("**1**"));
        assert!(b.contains("**6**"));
    }
}
