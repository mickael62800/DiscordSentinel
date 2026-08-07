use std::sync::Arc;

use atrium_proto::welcome::v1::{
    welcome_service_client::WelcomeServiceClient, ConversationScope, GenerateReplyRequest,
};
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready, guild::Member, id::ChannelId},
    prelude::*,
};
use tonic::transport::Channel;

mod logic;

#[derive(Clone)]
struct Config {
    token: String,
    grpc_url: String,
    general_channel_id: ChannelId,
    server_context: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            token: std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN manquant"),
            grpc_url: std::env::var("ATRIUM_GRPC_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8091".into()),
            general_channel_id: ChannelId::new(
                std::env::var("ATRIUM_GENERAL_CHANNEL_ID")
                    .expect("ATRIUM_GENERAL_CHANNEL_ID manquant")
                    .parse()
                    .expect("ATRIUM_GENERAL_CHANNEL_ID invalide"),
            ),
            server_context: std::env::var("ATRIUM_SERVER_CONTEXT").unwrap_or_default(),
        }
    }
}

struct Handler {
    config: Arc<Config>,
    channel: Channel,
}

impl Handler {
    async fn reply(
        &self,
        guild_id: String,
        member_id: String,
        name: String,
        channel_id: String,
        scope: ConversationScope,
        message: String,
    ) -> Option<String> {
        let mut client = WelcomeServiceClient::new(self.channel.clone());
        client
            .generate_reply(GenerateReplyRequest {
                guild_id,
                member_id,
                member_display_name: name,
                channel_id,
                scope: scope as i32,
                member_message: message,
                server_context: self.config.server_context.clone(),
            })
            .await
            .ok()
            .map(|response| response.into_inner().reply)
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        tracing::info!(user = %ready.user.name, "Atrium Bot pret");
    }

    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        if let Some(reply) = self
            .reply(
                member.guild_id.to_string(),
                member.user.id.to_string(),
                member.display_name().to_string(),
                self.config.general_channel_id.to_string(),
                ConversationScope::General,
                String::new(),
            )
            .await
        {
            if let Err(error) = self.config.general_channel_id.say(&ctx.http, reply).await {
                tracing::warn!(%error, "message d'accueil non envoye");
            }
        }
    }

    async fn message(&self, ctx: Context, message: Message) {
        if message.author.bot {
            return;
        }
        // Les MP sont une partie volontaire du parcours d'accueil. Dans le
        // general, le bot ne repond qu'a une mention pour eviter le spam.
        let is_direct = message.guild_id.is_none();
        let is_general = message.channel_id == self.config.general_channel_id;
        let is_mentioned =
            !is_direct && is_general && message.mentions_me(&ctx.http).await.unwrap_or(false);
        let scope = match logic::message_handling(is_direct, is_general, is_mentioned) {
            logic::MessageHandling::Ignore => return,
            logic::MessageHandling::Reply(scope) => scope,
        };
        let guild_id = message
            .guild_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "direct".into());
        if let Some(reply) = self
            .reply(
                guild_id,
                message.author.id.to_string(),
                message.author.display_name().to_string(),
                message.channel_id.to_string(),
                scope,
                message.content.clone(),
            )
            .await
        {
            if let Err(error) = message.channel_id.say(&ctx.http, reply).await {
                tracing::warn!(%error, "reponse Atrium non envoyee");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().init();
    let config = Arc::new(Config::from_env());
    let channel = Channel::from_shared(config.grpc_url.clone())
        .expect("ATRIUM_GRPC_URL invalide")
        .connect_lazy();
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&config.token, intents)
        .event_handler(Handler { config, channel })
        .await
        .expect("creation client Discord");
    client.start().await.expect("arret Atrium Bot");
}
