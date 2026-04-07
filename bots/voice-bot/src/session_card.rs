use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage, EditMessage};
use serenity::model::id::{ChannelId, MessageId};
use serenity::prelude::*;

/// Evenement dans l'historique d'une session vocale.
#[derive(Clone)]
pub struct SessionEvent {
    pub timestamp: String,
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
    pub created_at: String,
    /// Historique des evenements
    pub events: Vec<SessionEvent>,
    /// Nombre de membres actuellement presents
    pub current_members: u32,
    /// Salon termine ?
    pub closed: bool,
    pub closed_at: Option<String>,
    pub total_duration: Option<String>,
}

impl SessionCard {
    pub fn new(
        log_channel_id: ChannelId,
        creator_name: String,
        channel_type: String,
        created_at: String,
    ) -> Self {
        Self {
            log_message_id: None,
            log_channel_id,
            creator_name,
            channel_type,
            visibility: "Visible".to_string(),
            created_at,
            events: Vec::new(),
            current_members: 0,
            closed: false,
            closed_at: None,
            total_duration: None,
        }
    }

    pub fn add_event(&mut self, text: String) {
        let now = chrono::Utc::now().format("%H:%M").to_string();
        self.events.push(SessionEvent {
            timestamp: now,
            text,
        });
    }

    /// Construit l'embed Discord.
    pub fn build_embed(&self) -> CreateEmbed {
        let type_emoji = if self.channel_type == "private" { "\u{1f512}" } else { "\u{1f50a}" };
        let status_emoji = if self.closed { "\u{1f534}" } else { "\u{1f7e2}" };

        // Ligne header compacte
        let header = format!(
            "{} **{}** {} {} | {} | Cree a {}",
            type_emoji, self.creator_name, status_emoji,
            if self.channel_type == "private" { "Prive" } else { "Public" },
            self.visibility,
            self.created_at,
        );

        // Historique des evenements
        let mut history = String::new();
        if self.events.is_empty() {
            history.push_str("_En attente de membres..._");
        } else {
            for event in &self.events {
                history.push_str(&format!("`{}` {}\n", event.timestamp, event.text));
            }
        }

        // Tronquer si trop long (limite embed description 4096)
        if history.len() > 3500 {
            let keep = history.len() - 3000;
            history = format!("_... {} evenements precedents ..._\n{}", self.events.len() / 2, &history[keep..]);
        }

        // Footer
        let footer_text = if self.closed {
            format!(
                "Salon supprime {} | Duree : {}",
                self.closed_at.as_deref().unwrap_or(""),
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
            .footer(CreateEmbedFooter::new(footer_text))
            .timestamp(serenity::model::Timestamp::now())
    }

    /// Envoie la carte initiale dans le salon de logs.
    pub async fn send_initial(&mut self, ctx: &Context) {
        let embed = self.build_embed();
        let msg = CreateMessage::new().embed(embed);

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
