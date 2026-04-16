use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::errors::DomainError;
use crate::ports::outbound::{WelcomeConfigData, WelcomeConfigRepository};

pub struct PgWelcomeConfigRepository {
    pool: PgPool,
}

impl PgWelcomeConfigRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    guild_id: String,
    welcome_enabled: bool,
    welcome_channel_id: Option<String>,
    welcome_message: String,
    welcome_embed_color: String,
    welcome_dm_enabled: bool,
    welcome_dm_message: String,
    leave_enabled: bool,
    leave_channel_id: Option<String>,
    leave_message: String,
    rules_enabled: bool,
    rules_channel_id: Option<String>,
    rules_message: String,
    rules_role_id: Option<String>,
    rules_button_label: String,
    counter_enabled: bool,
    counter_channel_id: Option<String>,
    counter_format: String,
    anniversary_enabled: bool,
    anniversary_channel_id: Option<String>,
    anniversary_message: String,
    rejoin_message: String,
}

impl From<Row> for WelcomeConfigData {
    fn from(r: Row) -> Self {
        Self {
            guild_id: r.guild_id,
            welcome_enabled: r.welcome_enabled,
            welcome_channel_id: r.welcome_channel_id,
            welcome_message: r.welcome_message,
            welcome_embed_color: r.welcome_embed_color,
            welcome_dm_enabled: r.welcome_dm_enabled,
            welcome_dm_message: r.welcome_dm_message,
            leave_enabled: r.leave_enabled,
            leave_channel_id: r.leave_channel_id,
            leave_message: r.leave_message,
            rules_enabled: r.rules_enabled,
            rules_channel_id: r.rules_channel_id,
            rules_message: r.rules_message,
            rules_role_id: r.rules_role_id,
            rules_button_label: r.rules_button_label,
            counter_enabled: r.counter_enabled,
            counter_channel_id: r.counter_channel_id,
            counter_format: r.counter_format,
            anniversary_enabled: r.anniversary_enabled,
            anniversary_channel_id: r.anniversary_channel_id,
            anniversary_message: r.anniversary_message,
            rejoin_message: r.rejoin_message,
        }
    }
}

fn default_config(guild_id: &str) -> WelcomeConfigData {
    WelcomeConfigData {
        guild_id: guild_id.to_string(),
        welcome_enabled: true,
        welcome_channel_id: None,
        welcome_message: "Bienvenue {user} sur **{server}** ! Tu es le **{count}e** membre.".into(),
        welcome_embed_color: "3498db".into(),
        welcome_dm_enabled: false,
        welcome_dm_message: "Bienvenue sur **{server}** !".into(),
        leave_enabled: true,
        leave_channel_id: None,
        leave_message: "{user} nous a quittes. Nous sommes maintenant **{count}** membres.".into(),
        rules_enabled: false,
        rules_channel_id: None,
        rules_message: "Lis les regles et clique sur le bouton pour acceder au serveur.".into(),
        rules_role_id: None,
        rules_button_label: "J'accepte les regles".into(),
        counter_enabled: false,
        counter_channel_id: None,
        counter_format: "Membres : {count}".into(),
        anniversary_enabled: false,
        anniversary_channel_id: None,
        anniversary_message: "Felicitations {user}, ca fait **{years} an(s)** que tu es sur **{server}** !".into(),
        rejoin_message: "Content de te revoir {user} ! Tu nous avais manque.".into(),
    }
}

#[async_trait]
impl WelcomeConfigRepository for PgWelcomeConfigRepository {
    async fn get_config(&self, guild_id: &str) -> Result<WelcomeConfigData, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            "SELECT guild_id, welcome_enabled, welcome_channel_id, welcome_message, welcome_embed_color, \
             welcome_dm_enabled, welcome_dm_message, leave_enabled, leave_channel_id, leave_message, \
             rules_enabled, rules_channel_id, rules_message, rules_role_id, rules_button_label, \
             counter_enabled, counter_channel_id, counter_format, \
             anniversary_enabled, anniversary_channel_id, anniversary_message, rejoin_message \
             FROM welcome_config WHERE guild_id = $1",
        )
        .bind(guild_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(Into::into).unwrap_or_else(|| default_config(guild_id)))
    }
}
