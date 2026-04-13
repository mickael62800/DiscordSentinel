use serenity::builder::{CreateAttachment, CreateEmbed, CreateEmbedFooter, CreateMessage, EditMessage};
use serenity::model::id::{ChannelId, MessageId};
use serenity::prelude::*;

/// Evenement dans l'historique d'une session vocale.
/// `timestamp_unix` est un Unix timestamp (secondes depuis epoch, UTC).
/// On le formate en `<t:UNIX:t>` a l'affichage — Discord rend l'heure
/// dans le fuseau local du spectateur, quelle que soit la TZ du serveur.
#[derive(Clone)]
pub struct SessionEvent {
    pub timestamp_unix: i64,
    pub text: String,
}

/// Carte de session vocale vivante.
#[derive(Clone)]
pub struct SessionCard {
    /// ID du message embed dans le salon de logs
    pub log_message_id: Option<MessageId>,
    /// ID du salon de logs
    pub log_channel_id: ChannelId,
    /// Infos du salon
    pub creator_name: String,
    pub channel_type: String,
    pub visibility: String,
    /// Unix timestamp de creation (secondes UTC).
    pub created_at_unix: i64,
    /// Historique des evenements
    pub events: Vec<SessionEvent>,
    /// Nombre de membres actuellement presents
    pub current_members: u32,
    /// Salon termine ?
    pub closed: bool,
    /// Unix timestamp de fermeture (secondes UTC).
    pub closed_at_unix: Option<i64>,
    pub total_duration: Option<String>,
}

impl SessionCard {
    pub fn new(
        log_channel_id: ChannelId,
        creator_name: String,
        channel_type: String,
        created_at_unix: i64,
    ) -> Self {
        Self {
            log_message_id: None,
            log_channel_id,
            creator_name,
            channel_type,
            visibility: "Visible".to_string(),
            created_at_unix,
            events: Vec::new(),
            current_members: 0,
            closed: false,
            closed_at_unix: None,
            total_duration: None,
        }
    }

    pub fn add_event(&mut self, text: String) {
        self.events.push(SessionEvent {
            timestamp_unix: chrono::Utc::now().timestamp(),
            text,
        });
    }

    /// Construit l'embed Discord.
    pub fn build_embed(&self) -> CreateEmbed {
        let type_emoji = if self.channel_type == "private" { "\u{1f512}" } else { "\u{1f50a}" };
        let status_emoji = if self.closed { "\u{1f534}" } else { "\u{1f7e2}" };

        // Ligne header compacte. `<t:UNIX:t>` = heure courte (ex. 19:00)
        // dans le fuseau local du spectateur.
        let header = format!(
            "{} **{}** {} {} | {} | Cree a <t:{}:t>",
            type_emoji, self.creator_name, status_emoji,
            if self.channel_type == "private" { "Prive" } else { "Public" },
            self.visibility,
            self.created_at_unix,
        );

        // Historique des evenements
        let mut history = String::new();
        if self.events.is_empty() {
            history.push_str("_En attente de membres..._");
        } else {
            for event in &self.events {
                history.push_str(&format!("<t:{}:t> {}\n", event.timestamp_unix, event.text));
            }
        }

        // Tronquer si trop long (limite embed description 4096)
        if history.len() > 3500 {
            // Utiliser char_indices pour couper sur une frontiere UTF-8 valide
            let target_keep = history.len() - 3000;
            let safe_keep = history.char_indices()
                .map(|(i, _)| i)
                .find(|&i| i >= target_keep)
                .unwrap_or(target_keep);
            history = format!("_... {} evenements precedents ..._\n{}", self.events.len() / 2, &history[safe_keep..]);
        }

        // Footer — le markdown `<t:...:t>` n'est pas rendu ici, donc on
        // se contente de la duree. L'heure exacte de fermeture est visible
        // dans l'historique d'evenements au-dessus.
        let footer_text = if self.closed {
            format!(
                "Salon supprime | Duree : {}",
                self.total_duration.as_deref().unwrap_or("?"),
            )
        } else {
            format!("{} membre(s) en vocal", self.current_members)
        };

        let color = if self.closed {
            0x95a5a6u32 // Gris
        } else if self.current_members == 0 {
            0xf39c12 // Orange
        } else {
            0x2ecc71 // Vert
        };

        CreateEmbed::new()
            .description(format!("{}\n\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\n{}", header, history))
            .color(color)
            // Forcer la largeur maximale de l'embed via une image attachee
            .image("attachment://spacer.png")
            .footer(CreateEmbedFooter::new(footer_text))
            .timestamp(serenity::model::Timestamp::now())
    }

    /// Image transparente 1x1 PNG pour forcer l'embed en largeur max.
    fn spacer_attachment() -> CreateAttachment {
        // PNG 1x1 transparent minimal (68 bytes)
        const TRANSPARENT_PNG: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
            0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x62, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x01, 0xE5, 0x27, 0xDE, 0xFC, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
            0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        CreateAttachment::bytes(TRANSPARENT_PNG.to_vec(), "spacer.png")
    }

    /// Envoie la carte initiale dans le salon de logs.
    pub async fn send_initial(&mut self, ctx: &Context) {
        let embed = self.build_embed();
        let msg = CreateMessage::new().embed(embed).add_file(Self::spacer_attachment());

        match self.log_channel_id.send_message(&ctx.http, msg).await {
            Ok(message) => {
                self.log_message_id = Some(message.id);
            }
            Err(e) => {
                tracing::warn!(error = %e, "Erreur envoi carte session");
            }
        }
    }

    /// Met a jour la carte existante dans le salon de logs.
    /// Si le message initial n'a jamais ete envoye, tente de l'envoyer.
    pub async fn update(&mut self, ctx: &Context) {
        let message_id = match self.log_message_id {
            Some(id) => id,
            None => {
                tracing::info!("Carte session sans message_id, renvoi initial");
                self.send_initial(ctx).await;
                return;
            }
        };

        let embed = self.build_embed();
        let edit = EditMessage::new().embed(embed);

        if let Err(e) = self.log_channel_id.edit_message(&ctx.http, message_id, edit).await {
            tracing::warn!(error = %e, "Erreur mise a jour carte session");
        }
    }
}
