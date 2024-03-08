#![warn(clippy::pedantic)]
// They aren't exactly any more unreadable this way, even more so considering they are hexadecimal.
#![allow(clippy::unreadable_literal)]
// Preference.
#![allow(clippy::single_match, clippy::single_match_else)]
// Possibly will fix in the future, it just isn't a problem as it stands.
#![allow(clippy::too_many_lines)]

pub(crate) mod commands;
pub(crate) mod events;
pub(crate) mod formatting;
pub(crate) mod structures;

use std::sync::RwLock;
use std::time::Duration;
use std::{env, sync::Arc};

use octocrab::Octocrab;
use poise::serenity_prelude::{self as serenity, GatewayIntents};
use structures::BotState;

pub struct Data {
    pub octocrab: Arc<Octocrab>,
    pub state: RwLock<BotState>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {error:?}"),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {e}");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let discord_token = env::var("DISCORD_TOKEN").expect("Expected discord api token");
    let github_token = env::var("GITHUB_TOKEN").expect("Expected github api token");

    let octo_builder = Octocrab::builder().personal_token(github_token);

    let octocrab = octocrab::initialise(octo_builder).expect("Failed to build github client");

    let state = RwLock::new(BotState::read());

    let options = poise::FrameworkOptions {
        commands: vec![
            commands::register(),
            commands::snippets::snippet(),
            commands::snippets::create_snippet(),
            commands::snippets::remove_snippet(),
            commands::snippets::export_snippet(),
            commands::snippets::list_snippets(),
            commands::snippets::edit_snippet(),
            commands::utils::embed(),
            commands::utils::edit_embed(),
            commands::utils::add_repo(),
            commands::utils::remove_repo(),
            commands::utils::list_repos(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("!".into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(600),
            ))),
            ..Default::default()
        },
        on_error: |error| Box::pin(on_error(error)),

        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },

        skip_checks_for_owners: false,
        event_handler: |ctx, event: &serenity::FullEvent, framework, data| {
            Box::pin(events::event_handler(ctx, event, framework, data))
        },
        ..Default::default()
    };

    let framework = poise::Framework::new(options, move |ctx, ready, framework| {
        Box::pin(async move {
            println!("Logged in as {}", ready.user.name);
            poise::builtins::register_globally(ctx, &framework.options().commands).await?;
            Ok(Data { octocrab, state })
        })
    });

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::Client::builder(discord_token, intents)
        .framework(framework)
        .await
        .unwrap();

    client.start().await.unwrap();
}
