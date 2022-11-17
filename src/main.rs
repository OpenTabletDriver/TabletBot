#![feature(let_chains)]

pub(crate) mod structures;
pub(crate) mod commands;
pub(crate) mod events;
pub(crate) mod formatting;

use std::collections::HashSet;
use std::env;
use std::sync::Arc;
use octocrab::Octocrab;
use serenity::async_trait;
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::model::application::interaction::*;
use serenity::model::prelude::{Message, Ready, UserId, Reaction};
use serenity::prelude::*;
use crate::structures::*;

#[tokio::main]
async fn main() {
  let discord_token = env::var("DISCORD_TOKEN").expect("Export the 'DISCORD_TOKEN' environment variable");
  let github_token = env::var("GITHUB_TOKEN").expect("Export the 'GITHUB_TOKEN' environment variable");

  let http = Http::new(&discord_token);

  let (owners, bot_id) = match http.get_current_application_info().await {
    Ok(info) => {
      let mut owners = HashSet::new();
      owners.insert(info.owner.id);
      (owners, info.id)
    },
    Err(why) => panic!("Could not access application info: {:?}", why),
  };

  let framework = StandardFramework::new()
    .configure(|configuration| {
      configuration
        .on_mention(Some(UserId(*bot_id.as_u64())))
        .owners(owners)
        .prefix("!")
    });

  let intents = GatewayIntents::GUILD_MESSAGES
    | GatewayIntents::DIRECT_MESSAGES
    | GatewayIntents::GUILD_MESSAGE_REACTIONS
    | GatewayIntents::MESSAGE_CONTENT;

  let mut client = Client::builder(&discord_token, intents)
    .framework(framework)
    .event_handler(Handler)
    .await
    .expect("Error creating client");

  let octo_builder = Octocrab::builder()
    .personal_token(github_token);

  octocrab::initialise(octo_builder)
    .expect("Failed to build github client");

  {
    let mut data = client.data.write().await;
    data.insert::<ShardManagerContainer>(client.shard_manager.clone());
    data.insert::<State>(State::read().unwrap_or(State::default()));
  }

  let shard_manager = client.shard_manager.clone();
  tokio::spawn(async move {
    tokio::signal::ctrl_c().await.expect("Could not register ctrl+c handler");

    let arc = &Arc::from(http);
    if let Err(e) = commands::unregister(arc).await {
      println!("Failed to unregister commands prior to disconnecting: {}", e)
    }

    println!("Disconnecting");
    shard_manager.lock().await.shutdown_all().await;
  });

  if let Err(why) = client.start().await {
    println!("Client error: {:?}", why);
  }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
  async fn ready(&self, ctx: Context, ready: Ready) {
    println!("Connected to Discord API as bot user '{}#{:04}'", ready.user.name, ready.user.discriminator);

    match commands::unregister(&ctx.http).await {
      Ok(_) => {
        match commands::register(&ctx).await {
          Ok(_) => println!("Registered global commands "),
          Err(e) => println!("Failed to register global commands: {}", e.to_string())
        };
      },
      Err(e) => println!("Failed to register global commands: {}", e.to_string())
    }

    if let Ok(guilds) = ctx.http.get_guilds(None, None).await {
      for guild in guilds {
        if let Ok(app_commands) = guild.id.get_application_commands(&ctx).await {
          for command in app_commands {
            guild.id.delete_application_command(&ctx, command.id).await
              .expect(&format!("Failed to unregister guild command: {}", command.name));

            println!("Unregistered guild command {}", command.name);
          }
        }
      }
    }
  }

  async fn message(&self, ctx: Context, msg: Message) {
    let mut channel_name = "N/A".to_string();

    if let Ok(channel) = msg.channel(&ctx).await {
      if let Some(guild_channel) = channel.guild() {
        channel_name = guild_channel.name.clone();
      }
    }

    let user_name = format!("{}#{}", msg.author.name, msg.author.discriminator);
    println!("[#{}/{}]: {}", channel_name, user_name, msg.content);

    events::message(&ctx, &msg).await;
  }

  async fn reaction_add(&self, ctx: Context, added_reaction: Reaction) {
    events::reaction_add(&ctx, &added_reaction).await;
  }

  async fn reaction_remove(&self, ctx: Context, removed_reaction: Reaction) {
    events::reaction_remove(&ctx, &removed_reaction).await;
  }

  async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
    if let Interaction::ApplicationCommand(command) = interaction {
      println!("Received command interaction: {}", command.data.name);
      commands::interact(&ctx, &command).await;
    }
  }
}
