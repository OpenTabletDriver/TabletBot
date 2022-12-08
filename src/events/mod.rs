use serenity::model::prelude::Message;
use serenity::prelude::Context;

mod issue;
mod code;

pub async fn message(ctx: &Context, msg: &Message) {
  if !msg.author.bot {
    issue::message(&ctx, &msg).await;
    code::message(&ctx, &msg).await;
  }
}
