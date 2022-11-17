use serde::{Deserialize, Serialize};
use serenity::client::bridge::gateway::ShardManager;
use serenity::prelude::{TypeMapKey, Mutex};
use std::sync::Arc;

mod state;

pub use state::*;

#[derive(Deserialize, Serialize, Clone)]
pub struct ReactionRole {
  pub message_id: u64,
  pub role_id: u64,
  pub emote: String
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Snippet {
  pub id: String,
  pub title: String,
  pub content: String
}

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
  type Value = Arc<Mutex<ShardManager>>;
}
