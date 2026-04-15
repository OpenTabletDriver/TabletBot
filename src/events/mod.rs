use poise::serenity_prelude as serenity;

use crate::{Data, Error};

pub mod code;
pub mod issues;

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    data: &Data,
) -> Result<(), Error> {
    #[allow(clippy::single_match)]
    match event {
        serenity::FullEvent::Message { new_message } => {
            if !new_message.author.bot && new_message.guild_id.is_some() {
                issues::message(data, ctx, new_message).await;
                code::message(ctx, new_message).await;
            }
        }
        _ => (),
    }

    Ok(())
}
