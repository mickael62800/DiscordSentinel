//! Phase 7A.opt F.4 — Welcome config gRPC.
//!
//! Pas de use case unifie cote API (welcome_config est une table adhoc),
//! donc on copie la logique du handler HTTP `welcome.rs` avec sqlx direct.

use tonic::{Request, Response, Status};

use sentinel_proto::welcome::v1 as proto;
use sentinel_proto::welcome::v1::welcome_service_server::WelcomeService;

pub struct WelcomeGrpc {
    pub pg_pool: sqlx::PgPool,
}

#[derive(sqlx::FromRow)]
struct WelcomeConfigRow {
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

impl WelcomeConfigRow {
    fn default_for(guild_id: &str) -> Self {
        Self {
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
}

#[tonic::async_trait]
impl WelcomeService for WelcomeGrpc {
    async fn get_config(
        &self,
        request: Request<proto::GetConfigRequest>,
    ) -> Result<Response<proto::WelcomeConfig>, Status> {
        let req = request.into_inner();
        let row = sqlx::query_as::<_, WelcomeConfigRow>(
            "SELECT guild_id, welcome_enabled, welcome_channel_id, welcome_message, welcome_embed_color, \
             welcome_dm_enabled, welcome_dm_message, leave_enabled, leave_channel_id, leave_message, \
             rules_enabled, rules_channel_id, rules_message, rules_role_id, rules_button_label, \
             counter_enabled, counter_channel_id, counter_format, \
             anniversary_enabled, anniversary_channel_id, anniversary_message, rejoin_message \
             FROM welcome_config WHERE guild_id = $1",
        )
        .bind(&req.guild_id)
        .fetch_optional(&self.pg_pool)
        .await
        .map_err(|e| Status::internal(format!("SELECT welcome_config: {e}")))?
        .unwrap_or_else(|| WelcomeConfigRow::default_for(&req.guild_id));

        Ok(Response::new(proto::WelcomeConfig {
            guild_id: row.guild_id,
            welcome_enabled: row.welcome_enabled,
            welcome_channel_id: row.welcome_channel_id,
            welcome_message: row.welcome_message,
            welcome_embed_color: row.welcome_embed_color,
            welcome_dm_enabled: row.welcome_dm_enabled,
            welcome_dm_message: row.welcome_dm_message,
            leave_enabled: row.leave_enabled,
            leave_channel_id: row.leave_channel_id,
            leave_message: row.leave_message,
            rules_enabled: row.rules_enabled,
            rules_channel_id: row.rules_channel_id,
            rules_message: row.rules_message,
            rules_role_id: row.rules_role_id,
            rules_button_label: row.rules_button_label,
            counter_enabled: row.counter_enabled,
            counter_channel_id: row.counter_channel_id,
            counter_format: row.counter_format,
            anniversary_enabled: row.anniversary_enabled,
            anniversary_channel_id: row.anniversary_channel_id,
            anniversary_message: row.anniversary_message,
            rejoin_message: row.rejoin_message,
        }))
    }
}
