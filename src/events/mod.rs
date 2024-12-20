use poise::serenity_prelude as serenity;

use crate::{Data, Error};

pub mod code;
pub mod config_creation;
pub mod issues;

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Message { new_message } => {
            if !new_message.author.bot && new_message.guild_id.is_some() {
                issues::message(data, ctx, new_message).await;
                code::message(ctx, new_message).await;
            }
        }
        serenity::FullEvent::ThreadCreate { thread } => {
            config_creation::thread_create(ctx, data, thread).await;
        }
        _ => (),
    }

    Ok(())
}
