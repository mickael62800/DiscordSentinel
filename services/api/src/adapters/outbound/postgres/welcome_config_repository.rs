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
            // Row ne contient pas les champs embed enrichi (legacy welcome_config
            // table, desormais inutilisee en lecture). Valeurs par defaut.
            welcome_title: "Bienvenue !".into(),
            welcome_image_url: "".into(),
            welcome_footer_text: "{count} membres".into(),
            leave_title: "Au revoir...".into(),
            leave_image_url: "".into(),
            leave_footer_text: "{count} membres".into(),
            anniversary_title: "Joyeux anniversaire !".into(),
            anniversary_image_url: "".into(),
            anniversary_footer_text: "{count} membres".into(),
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
        welcome_title: "Bienvenue !".into(),
        welcome_image_url: "".into(),
        welcome_footer_text: "{count} membres".into(),
        leave_title: "Au revoir...".into(),
        leave_image_url: "".into(),
        leave_footer_text: "{count} membres".into(),
        anniversary_title: "Joyeux anniversaire !".into(),
        anniversary_image_url: "".into(),
        anniversary_footer_text: "{count} membres".into(),
    }
}

fn parse_bool(v: &str, default: bool) -> bool {
    matches!(v, "true" | "1" | "yes" | "on") || (v.is_empty() && default)
}

fn overlay_with_bot_config(
    base: WelcomeConfigData,
    kvs: Vec<(String, String)>,
) -> WelcomeConfigData {
    let mut d = base;
    for (k, v) in kvs {
        match k.as_str() {
            "welcome_enabled" => d.welcome_enabled = parse_bool(&v, d.welcome_enabled),
            "welcome_channel_id" => d.welcome_channel_id = if v.is_empty() { None } else { Some(v) },
            "welcome_message" => { if !v.is_empty() { d.welcome_message = v; } }
            "welcome_embed_color" => { if !v.is_empty() { d.welcome_embed_color = v; } }
            "welcome_dm_enabled" => d.welcome_dm_enabled = parse_bool(&v, d.welcome_dm_enabled),
            "welcome_dm_message" => { if !v.is_empty() { d.welcome_dm_message = v; } }
            "rejoin_message" => { if !v.is_empty() { d.rejoin_message = v; } }
            "leave_enabled" => d.leave_enabled = parse_bool(&v, d.leave_enabled),
            "leave_channel_id" => d.leave_channel_id = if v.is_empty() { None } else { Some(v) },
            "leave_message" => { if !v.is_empty() { d.leave_message = v; } }
            "rules_enabled" => d.rules_enabled = parse_bool(&v, d.rules_enabled),
            "rules_channel_id" => d.rules_channel_id = if v.is_empty() { None } else { Some(v) },
            "rules_message" => { if !v.is_empty() { d.rules_message = v; } }
            "rules_role_id" => d.rules_role_id = if v.is_empty() { None } else { Some(v) },
            "rules_button_label" => { if !v.is_empty() { d.rules_button_label = v; } }
            "counter_enabled" => d.counter_enabled = parse_bool(&v, d.counter_enabled),
            "counter_channel_id" => d.counter_channel_id = if v.is_empty() { None } else { Some(v) },
            "counter_format" => { if !v.is_empty() { d.counter_format = v; } }
            "anniversary_enabled" => d.anniversary_enabled = parse_bool(&v, d.anniversary_enabled),
            "anniversary_channel_id" => d.anniversary_channel_id = if v.is_empty() { None } else { Some(v) },
            "anniversary_message" => { if !v.is_empty() { d.anniversary_message = v; } }
            "welcome_title" => { if !v.is_empty() { d.welcome_title = v; } }
            "welcome_image_url" => d.welcome_image_url = v,
            "welcome_footer_text" => { if !v.is_empty() { d.welcome_footer_text = v; } }
            "leave_title" => { if !v.is_empty() { d.leave_title = v; } }
            "leave_image_url" => d.leave_image_url = v,
            "leave_footer_text" => { if !v.is_empty() { d.leave_footer_text = v; } }
            "anniversary_title" => { if !v.is_empty() { d.anniversary_title = v; } }
            "anniversary_image_url" => d.anniversary_image_url = v,
            "anniversary_footer_text" => { if !v.is_empty() { d.anniversary_footer_text = v; } }
            _ => {}
        }
    }
    d
}

#[async_trait]
impl WelcomeConfigRepository for PgWelcomeConfigRepository {
    /// Lit la config welcome depuis `bot_guild_config` (migration 148).
    /// Fallback sur les defaults si aucune cle n est configuree pour ce
    /// serveur. L ancienne table `welcome_config` n est plus lue.
    async fn get_config(&self, guild_id: &str) -> Result<WelcomeConfigData, DomainError> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT config_key, config_value FROM bot_guild_config \
             WHERE guild_id = $1 AND bot_name = 'welcome-bot'",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(overlay_with_bot_config(default_config(guild_id), rows))
    }

    async fn save_config(&self, guild_id: &str, d: &WelcomeConfigData) -> Result<WelcomeConfigData, DomainError> {
        let row: Row = sqlx::query_as(
            r#"INSERT INTO welcome_config (guild_id,
                welcome_enabled, welcome_channel_id, welcome_message, welcome_embed_color,
                welcome_dm_enabled, welcome_dm_message,
                leave_enabled, leave_channel_id, leave_message,
                rules_enabled, rules_channel_id, rules_message, rules_role_id, rules_button_label,
                counter_enabled, counter_channel_id, counter_format,
                anniversary_enabled, anniversary_channel_id, anniversary_message, rejoin_message)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)
               ON CONFLICT (guild_id) DO UPDATE SET
                welcome_enabled = COALESCE($2, welcome_config.welcome_enabled),
                welcome_channel_id = COALESCE($3, welcome_config.welcome_channel_id),
                welcome_message = COALESCE($4, welcome_config.welcome_message),
                welcome_embed_color = COALESCE($5, welcome_config.welcome_embed_color),
                welcome_dm_enabled = COALESCE($6, welcome_config.welcome_dm_enabled),
                welcome_dm_message = COALESCE($7, welcome_config.welcome_dm_message),
                leave_enabled = COALESCE($8, welcome_config.leave_enabled),
                leave_channel_id = COALESCE($9, welcome_config.leave_channel_id),
                leave_message = COALESCE($10, welcome_config.leave_message),
                rules_enabled = COALESCE($11, welcome_config.rules_enabled),
                rules_channel_id = COALESCE($12, welcome_config.rules_channel_id),
                rules_message = COALESCE($13, welcome_config.rules_message),
                rules_role_id = COALESCE($14, welcome_config.rules_role_id),
                rules_button_label = COALESCE($15, welcome_config.rules_button_label),
                counter_enabled = COALESCE($16, welcome_config.counter_enabled),
                counter_channel_id = COALESCE($17, welcome_config.counter_channel_id),
                counter_format = COALESCE($18, welcome_config.counter_format),
                anniversary_enabled = COALESCE($19, welcome_config.anniversary_enabled),
                anniversary_channel_id = COALESCE($20, welcome_config.anniversary_channel_id),
                anniversary_message = COALESCE($21, welcome_config.anniversary_message),
                rejoin_message = COALESCE($22, welcome_config.rejoin_message),
                updated_at = NOW()
               RETURNING guild_id, welcome_enabled, welcome_channel_id, welcome_message, welcome_embed_color,
                welcome_dm_enabled, welcome_dm_message, leave_enabled, leave_channel_id, leave_message,
                rules_enabled, rules_channel_id, rules_message, rules_role_id, rules_button_label,
                counter_enabled, counter_channel_id, counter_format,
                anniversary_enabled, anniversary_channel_id, anniversary_message, rejoin_message"#,
        )
        .bind(guild_id)
        .bind(d.welcome_enabled).bind(&d.welcome_channel_id).bind(&d.welcome_message).bind(&d.welcome_embed_color)
        .bind(d.welcome_dm_enabled).bind(&d.welcome_dm_message)
        .bind(d.leave_enabled).bind(&d.leave_channel_id).bind(&d.leave_message)
        .bind(d.rules_enabled).bind(&d.rules_channel_id).bind(&d.rules_message).bind(&d.rules_role_id).bind(&d.rules_button_label)
        .bind(d.counter_enabled).bind(&d.counter_channel_id).bind(&d.counter_format)
        .bind(d.anniversary_enabled).bind(&d.anniversary_channel_id).bind(&d.anniversary_message)
        .bind(&d.rejoin_message)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.into())
    }
}
