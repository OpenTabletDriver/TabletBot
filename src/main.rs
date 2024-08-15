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

use std::sync::atomic::AtomicBool;

pub struct Data {
    pub has_started: AtomicBool,
    pub octocrab: Octocrab,
    pub state: RwLock<BotState>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
type FrameworkContext<'a> = poise::FrameworkContext<'a, Data, Error>;

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { ctx, error, .. } => {
            let error = error.to_string();
            eprintln!("An error occured in a command: {error}");
            commands::respond_err(&ctx, "Command Error", &error).await;
        }

        poise::FrameworkError::ArgumentParse {
            error, input, ctx, ..
        } => {
            let usage = match &ctx.command().help_text {
                Some(help_text) => &**help_text,
                None => "Please check the help menu for usage information",
            };
            let response = if let Some(input) = input {
                format!("**Cannot parse `{input}` as argument: {error}**\n{usage}")
            } else {
                format!("### {error}\n{usage}")
            };
            commands::respond_err(&ctx, "Argument Parsing Error", &response).await;
        }
        poise::FrameworkError::GuildOnly { ctx, .. } => {
            commands::respond_err(
                &ctx,
                "This command cannot be ran in DMs.",
                "You cannot run this command in DMs.",
            )
            .await;
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

    let octocrab = octo_builder.build().expect("Failed to build github client");

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
            commands::udev::generate_udev(),
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
        event_handler: |framework, event| Box::pin(events::event_handler(framework, event)),
        ..Default::default()
    };

    let data = Data {
        has_started: AtomicBool::new(false),
        octocrab,
        state,
    };

    let framework = poise::Framework::new(options);

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::Client::builder(&discord_token, intents)
        .framework(framework)
        .data(Arc::new(data))
        .await
        .unwrap();

    client.start().await.unwrap();
}
