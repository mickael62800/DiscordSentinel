use serenity::all::{CommandInteraction, Context, CreateCommand};

use sentinel_shared::discord_helpers::reply_ephemeral;

pub fn register() -> CreateCommand {
    CreateCommand::new("blackjack")
        .description("Jouer au Blackjack")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    reply_ephemeral(ctx, command, "Blackjack arrive bientot !").await;
}
