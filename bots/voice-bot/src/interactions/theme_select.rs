use serenity::builder::{
    CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
};
use serenity::model::application::{ComponentInteraction, ComponentInteractionDataKind};
use serenity::model::id::GuildId;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::heartbeat::ApiClientKey;

use crate::api_client::{ApiClient, ThemeResponse};

/// Handle theme selection interactions.
pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();

    if custom_id.starts_with("theme_select_") {
        handle_theme_selected(ctx, component).await;
    } else {
        warn!(custom_id = %custom_id, "Theme select interaction inconnue");
    }
}

/// User selected a theme from the dropdown menu.
async fn handle_theme_selected(ctx: &Context, component: &ComponentInteraction) {
    let selected_theme_id = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            if values.is_empty() {
                super::respond_ephemeral(ctx, component, "Aucun theme selectionne.").await;
                return;
            }
            values[0].clone()
        }
        _ => {
            super::respond_ephemeral(ctx, component, "Erreur: type de composant inattendu.").await;
            return;
        }
    };

    // Parse custom_id: theme_select_{guild_id}_{user_id}_{kind}
    let parts: Vec<&str> = component.data.custom_id.splitn(5, '_').collect();
    if parts.len() < 5 {
        super::respond_ephemeral(ctx, component, "Erreur: format d'interaction invalide.").await;
        return;
    }

    let guild_id_str = parts[2];
    let expected_user_id = parts[3];
    let kind = parts[4];

    // Verifier que c'est le bon utilisateur qui repond
    if component.user.id.get().to_string() != expected_user_id {
        super::respond_ephemeral(ctx, component, "Cette selection n'est pas pour toi.").await;
        return;
    }

    let guild_id = GuildId::new(guild_id_str.parse().unwrap_or(0));
    if guild_id.get() == 0 {
        super::respond_ephemeral(ctx, component, "Erreur: guild invalide.").await;
        return;
    }

    let user_id = component.user.id;

    // Fetch the selected theme
    let theme = {
        let data = ctx.data.read().await;
        let Some(base) = data.get::<ApiClientKey>() else {
            super::respond_ephemeral(ctx, component, "Erreur interne (API client).").await;
            return;
        };
        let api = ApiClient::new(base.clone());
        match api.list_themes(&guild_id.to_string()).await {
            Ok(themes) => themes.into_iter().find(|t| t.id == selected_theme_id),
            Err(e) => {
                error!(error = %e, "Erreur chargement themes");
                None
            }
        }
    };

    // Acknowledge the interaction
    super::respond_ephemeral(
        ctx,
        component,
        &format!(
            "Theme **{}** selectionne ! Creation du salon...",
            theme.as_ref().map(|t| t.name.as_str()).unwrap_or("Par defaut")
        ),
    )
    .await;

    // Create the channel with the theme
    crate::handlers::voice::create_temp_channel_with_theme(
        ctx,
        guild_id,
        user_id,
        kind,
        theme,
    )
    .await;

    info!(user = %user_id, theme_id = %selected_theme_id, kind = %kind, "Salon cree avec theme");
}

/// Build a theme selection menu for the given themes.
#[allow(dead_code)]
pub fn build_theme_menu(
    themes: &[ThemeResponse],
    guild_id: u64,
    user_id: u64,
    kind: &str,
) -> CreateSelectMenu {
    let options: Vec<CreateSelectMenuOption> = themes
        .iter()
        .map(|t| {
            let desc = format!(
                "Limite: {} | {}",
                t.member_limit.map(|l| l.to_string()).unwrap_or_else(|| "illimite".into()),
                t.visibility,
            );
            let mut opt = CreateSelectMenuOption::new(&t.name, &t.id).description(&desc);
            if let Some(ref emoji) = t.emoji {
                if emoji.len() <= 4 {
                    opt = opt.emoji(serenity::model::channel::ReactionType::Unicode(emoji.clone()));
                }
            }
            if t.is_default {
                opt = opt.default_selection(true);
            }
            opt
        })
        .collect();

    CreateSelectMenu::new(
        format!("theme_select_{guild_id}_{user_id}_{kind}"),
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Choisir un theme pour le salon")
    .min_values(1)
    .max_values(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_client::ThemeResponse;

    fn make_theme(id: &str, name: &str, is_default: bool) -> ThemeResponse {
        ThemeResponse {
            id: id.to_string(),
            name: name.to_string(),
            emoji: None,
            channel_name_template: "{user}".to_string(),
            member_limit: None,
            visibility: "visible".to_string(),
            locked: false,
            queue_enabled: false,
            bitrate: None,
            slowmode_secs: None,
            stage_enabled: false,
            is_default,
        }
    }

    #[test]
    fn build_menu_with_single_theme() {
        let themes = vec![make_theme("t1", "Gaming", false)];
        let menu = build_theme_menu(&themes, 123, 456, "private");
        // Menu should be created without panic
        let _ = menu;
    }

    #[test]
    fn build_menu_with_multiple_themes() {
        let themes = vec![
            make_theme("t1", "Gaming", false),
            make_theme("t2", "Musique", true),
            make_theme("t3", "Travail", false),
        ];
        let menu = build_theme_menu(&themes, 111, 222, "public");
        let _ = menu;
    }

    #[test]
    fn build_menu_with_emoji() {
        let mut theme = make_theme("t1", "Gaming", false);
        theme.emoji = Some("🎮".to_string());
        let themes = vec![theme];
        let menu = build_theme_menu(&themes, 123, 456, "private");
        let _ = menu;
    }

    #[test]
    fn build_menu_with_member_limit() {
        let mut theme = make_theme("t1", "Duo", false);
        theme.member_limit = Some(2);
        let themes = vec![theme];
        let menu = build_theme_menu(&themes, 123, 456, "private");
        let _ = menu;
    }

    #[test]
    fn build_menu_empty_themes() {
        let themes: Vec<ThemeResponse> = vec![];
        let menu = build_theme_menu(&themes, 123, 456, "private");
        let _ = menu;
    }

    #[test]
    fn build_menu_long_emoji_skipped() {
        let mut theme = make_theme("t1", "Test", false);
        theme.emoji = Some("this_is_too_long".to_string());
        let themes = vec![theme];
        // Should not panic even with long emoji (just skips it)
        let menu = build_theme_menu(&themes, 123, 456, "private");
        let _ = menu;
    }
}
