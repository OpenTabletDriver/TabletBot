use serenity::model::prelude::{Message, Reaction};
use serenity::prelude::Context;

mod code;
mod issue;
mod role;

pub async fn message(ctx: &Context, msg: &Message) {
  issue::message(ctx, msg).await;
  code::message(ctx, msg).await;
}

pub async fn reaction_add(ctx: &Context, added_reaction: &Reaction) {
  role::reaction_add(ctx, added_reaction).await;
}

pub async fn reaction_remove(ctx: &Context, removed_reaction: &Reaction) {
  role::reaction_remove(ctx, removed_reaction).await;
}
