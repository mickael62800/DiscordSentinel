use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::api_client::{Action, AnalyzeRequest, ApiClient, MessageMetadata};
use crate::detectors;

/// Clé pour accéder à l'ApiClient dans le TypeMap de Serenity.
pub struct ApiClientKey;

impl TypeMapKey for ApiClientKey {
    type Value = ApiClient;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        // Ignorer les messages de bots
        if msg.author.bot {
            return;
        }

        let content = &msg.content;

        // Analyse locale (détection rapide)
        let flags = detectors::analyze(content);

        // Si rien détecté, on ne sollicite pas le backend
        if !flags.spam && !flags.insult && !flags.link {
            return;
        }

        info!(
            guild_id = ?msg.guild_id,
            user = %msg.author.name,
            flags.spam = flags.spam,
            flags.insult = flags.insult,
            flags.link = flags.link,
            "Message flaggé"
        );

        // Construire la requête pour le backend
        let request = AnalyzeRequest {
            guild_id: msg.guild_id.map(|id| id.to_string()).unwrap_or_default(),
            channel_id: msg.channel_id.to_string(),
            user_id: msg.author.id.to_string(),
            username: msg.author.name.clone(),
            content: content.clone(),
            flags,
            metadata: MessageMetadata {
                message_id: msg.id.to_string(),
                timestamp: msg.timestamp.to_string(),
            },
        };

        // Envoyer au backend
        let data = ctx.data.read().await;
        let api_client = match data.get::<ApiClientKey>() {
            Some(client) => client,
            None => {
                error!("ApiClient introuvable dans le contexte");
                return;
            }
        };

        match api_client.analyze(&request).await {
            Ok(response) => {
                info!(action = ?response.action, reason = ?response.reason, "Réponse du backend");

                if let Err(e) = execute_action(&ctx, &msg, &response.action, response.reason.as_deref()).await {
                    error!(error = %e, "Erreur lors de l'exécution de l'action");
                }
            }
            Err(e) => {
                warn!(error = %e, "Backend injoignable — action locale par défaut");
                // Fallback : supprimer le message si insulte détectée
                if request.flags.insult {
                    if let Err(e) = msg.delete(&ctx.http).await {
                        error!(error = %e, "Impossible de supprimer le message");
                    }
                }
            }
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Automod bot connecté");
    }
}

/// Exécute l'action décidée par le backend.
async fn execute_action(
    ctx: &Context,
    msg: &Message,
    action: &Action,
    reason: Option<&str>,
) -> Result<(), serenity::Error> {
    let reason_text = reason.unwrap_or("Automod");

    match action {
        Action::None => {}
        Action::Delete => {
            msg.delete(&ctx.http).await?;
            info!(message_id = %msg.id, "Message supprimé");
        }
        Action::Warn => {
            msg.reply(&ctx.http, format!("⚠️ Avertissement : {reason_text}"))
                .await?;
        }
        Action::Mute => {
            msg.delete(&ctx.http).await?;
            if let (Some(guild_id), Ok(member)) = (msg.guild_id, msg.member(&ctx.http).await) {
                let mut member = guild_id.member(&ctx.http, member.user.id).await?;
                let secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
                    + 600; // 10 minutes par défaut
                let datetime = time::OffsetDateTime::from_unix_timestamp(secs)
                    .expect("timestamp invalide");
                let timeout = serenity::model::Timestamp::from(datetime);
                member
                    .disable_communication_until_datetime(&ctx.http, timeout)
                    .await?;
                info!(user = %msg.author.name, "Utilisateur mute (10 min)");
            }
        }
        Action::Ban => {
            if let Some(guild_id) = msg.guild_id {
                guild_id
                    .ban_with_reason(&ctx.http, msg.author.id, 1, reason_text)
                    .await?;
                info!(user = %msg.author.name, "Utilisateur banni");
            }
        }
    }

    Ok(())
}
