use poise::serenity_prelude as serenity;

use crate::{Data, Error};

pub mod code;
pub mod issues;

pub struct Handler;

#[serenity::async_trait]
impl serenity::EventHandler for Handler {
    async fn dispatch(&self, ctx: &serenity::Context, event: &serenity::FullEvent) {
        let _ = event_handler(ctx, event).await;
    }
}

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            let data = ctx.data_ref::<Data>();
            let shard_count = ctx.cache.shard_count();
            let is_last_shard = (ctx.shard_id.0 + 1) == shard_count.get();

            if is_last_shard
                && !data
                    .has_started
                    .swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                println!("Logged in as {}", data_about_bot.user.tag());
            }
        }
        serenity::FullEvent::Message { new_message, .. } => {
            if !new_message.author.bot() && new_message.guild_id.is_some() {
                issues::message(ctx, new_message).await;
                code::message(ctx, new_message).await;
            }
        }
        _ => (),
    }

    Ok(())
}
