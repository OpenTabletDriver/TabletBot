use poise::serenity_prelude as serenity;

use crate::Error;

pub mod code;
pub mod issues;

pub struct Handler;

#[serenity::async_trait]
impl serenity::EventHandler for Handler {
    async fn dispatch(&self, ctx: &serenity::Context, event: &serenity::FullEvent) {
        let _ = event_handler(&ctx, event).await;
    }
}

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Message { new_message , .. } => {
            if !new_message.author.bot() && new_message.guild_id.is_some() {
                issues::message(ctx, new_message).await;
                code::message(ctx, new_message).await;
            }
        }
        _ => (),
    }

    Ok(())
}
