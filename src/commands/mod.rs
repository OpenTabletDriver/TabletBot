use core::panic;
use serenity::builder::CreateApplicationCommand;
use serenity::Error;
use serenity::http::Http;
use serenity::model::prelude::command::Command;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::interaction::application_command::CommandDataOptionValue;
use serenity::prelude::Context;
use serenity::utils::Colour;
use std::sync::Arc;
use crate::commands::reaction::*;
use crate::commands::snippets::*;
use crate::commands::types::*;

mod reaction;
mod snippets;
mod types;

pub const OK_COLOR: Colour = Colour(0x2ecc71);
pub const ERROR_COLOR: Colour = Colour(0xe74c3c);

pub async fn get_commands(ctx: &Context) -> Vec<CreateApplicationCommand> {
  vec![
    SnippetCommand::register(ctx).await,
    SetSnippetCommand::register(ctx).await,
    RemoveSnippetCommand::register(ctx).await,
    ExportSnippetCommand::register(ctx).await,
    AddReactionRoleCommand::register(ctx).await,
    RemoveReactionRoleCommand::register(ctx).await
  ]
}

pub async fn register(ctx: &Context) -> Result<Vec<Command>, Error> {
  let commands = get_commands(ctx).await;

  Command::set_global_application_commands(&ctx.http, |c|
    c.set_application_commands(commands)
  ).await
}

pub async fn unregister(http: &Arc<Http>) -> Result<(), Error> {
  if let Ok(commands) = Command::get_global_application_commands(http).await {
    for command in commands {
      if let Err(e) = Command::delete_global_application_command(http, command.id).await {
        return Err(e);
      }
    }
  }

  Ok(())
}

pub async fn interact(ctx: &Context, command: &ApplicationCommandInteraction) {
  match command.data.name.as_str() {
    SNIPPET_NAME => SnippetCommand::invoke(ctx, command),
    SET_SNIPPET_NAME => SetSnippetCommand::invoke(ctx, command),
    REMOVE_SNIPPET_NAME => RemoveSnippetCommand::invoke(ctx, command),
    EXPORT_SNIPPET_NAME => ExportSnippetCommand::invoke(ctx, command),
    ADD_REACTION_ROLE_NAME => AddReactionRoleCommand::invoke(ctx, command),
    REMOVE_REACTION_ROLE_NAME => RemoveReactionRoleCommand::invoke(ctx, command),
    _ => panic!("Invalid interaction command: {}", command.data.name)
  }.await
}

pub fn get_option_value(command: &ApplicationCommandInteraction, index: usize, name: &str) -> CommandDataOptionValue {
  command.data.options.get(index)
    .expect(&format!("Expected {} for argument {}", name, index))
    .resolved
    .as_ref()
    .expect(&format!("Expected {} for argument {}", name, index))
    .clone()
}
