pub(crate) mod structures;
pub(crate) mod commands;
pub(crate) mod events;
pub(crate) mod formatting;

use octocrab::Octocrab;
use serenity::async_trait;
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::model::application::interaction::*;
use serenity::model::prelude::{Message, Ready, UserId};
use serenity::prelude::*;
use std::collections::HashSet;
use std::env;
use crate::structures::*;

#[tokio::main]
async fn main() {
  let discord_token = env::var("DISCORD_TOKEN").expect("Expected discord api token");
  let github_token = env::var("GITHUB_TOKEN").expect("Expected github api token");

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
    data.insert::<State>(State::read());
    data.insert::<ShardManagerContainer>(client.shard_manager.clone());
  }

  let shard_manager = client.shard_manager.clone();
  tokio::spawn(async move {
    tokio::signal::ctrl_c().await.expect("Could not register ctrl+c handler");

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

    commands::register(&ctx).await;
  }

  async fn message(&self, ctx: Context, msg: Message) {
    let channel_name = msg.channel(&ctx).await.ok()
      .and_then(|c| c.guild())
      .and_then(|c| Some(c.name))
      .unwrap_or("N/A".to_string());

    let user_name = format!("{}#{}", msg.author.name, msg.author.discriminator);
    let mut content = msg.content.clone();

    let embeds_len = msg.embeds.len();

    let suffix = match embeds_len {
      2.. => Some("embeds"),
      1 => Some("embed"),
      _ => None
    };

    if let Some(suffix) = suffix {
      content += &format!("({} {})", embeds_len, suffix);
    }

    println!("[#{channel_name}/{user_name}]: {content}");

    events::message(&ctx, &msg).await;
  }

  async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
    if let Interaction::ApplicationCommand(command) = interaction {
      println!("Received command interaction '{}'", command.data.name);
      commands::interact(&ctx, &command).await;
    }
  }
}
