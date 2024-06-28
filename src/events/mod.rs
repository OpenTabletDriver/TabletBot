use std::sync::atomic::Ordering;

use poise::serenity_prelude as serenity;

use crate::{Error, FrameworkContext};

pub mod code;
pub mod issues;

pub async fn event_handler(
    framework: FrameworkContext<'_>,
    event: &serenity::FullEvent,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Message { new_message } => {
            if !new_message.author.bot() && new_message.guild_id.is_some() {
                issues::message(framework, new_message).await;
                code::message(framework, new_message).await;
            }
        }
        serenity::FullEvent::Ready { data_about_bot } => {
            if !framework
                .user_data()
                .has_started
                .swap(true, Ordering::SeqCst)
            {
                finalize_startup(framework, data_about_bot).await?;
            }
        }
        _ => (),
    }

    Ok(())
}

// startup function that will only be ran once.
async fn finalize_startup(
    framework: FrameworkContext<'_>,
    ready: &serenity::Ready,
) -> Result<(), Error> {
    println!("Logged in as {}", ready.user.name);
    poise::builtins::register_globally(
        &framework.serenity_context.http,
        &framework.options().commands,
    )
    .await?;
    Ok(())
}
