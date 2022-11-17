use serenity::async_trait;
use serenity::builder::{CreateApplicationCommand, CreateApplicationCommandOption};
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue};
use serenity::prelude::Context;
use crate::commands::{get_option_value, GenOpts, SlashCommand};
use crate::structures::{Snippet, State};

async fn followup_snippet(ctx: &Context, state: &State, command: &ApplicationCommandInteraction, snippet_name: String) {
  let snippet = state.snippets
    .iter()
    .find(|s| snippet_name.eq(&s.id));

  if let Some(snippet) = snippet {
    command.create_followup_message(&ctx.http, |f|
      f.embed(|e| e
        .title(&snippet.title)
        .description(&snippet.content)
        .color(crate::commands::OK_COLOR)
      )
    ).await.expect("Failed to respond with snippet embed");
  } else {
    command.create_followup_message(&ctx.http, |f|
      f.embed(|e| e
        .title("Error")
        .description(format!("Failed to get embed for snippet '{}'", snippet_name))
        .color(crate::commands::ERROR_COLOR)
      )
    ).await.expect("Failed to respond with error embed");
  }
}

impl GenOpts for CreateApplicationCommandOption {
  fn gen_opts(&mut self, state: &State) -> &mut Self {
    for snippet in state.snippets[..25].iter() {
      let name = format!("{}: {}", snippet.id, snippet.title);
      self.add_string_choice(name, snippet.id.to_string());
    }

    self
  }
}

pub const SNIPPET_NAME: &str = "snippet";
pub struct SnippetCommand;

pub const SET_SNIPPET_NAME: &str = "set-snippet";
pub struct SetSnippetCommand;

pub const REMOVE_SNIPPET_NAME: &str = "remove-snippet";
pub struct RemoveSnippetCommand;

pub const EXPORT_SNIPPET_NAME: &str = "export-snippet";
pub struct ExportSnippetCommand;

#[async_trait]
impl SlashCommand for SnippetCommand {
  async fn register(ctx: &Context) -> CreateApplicationCommand {
    let data = ctx.data.read().await;
    let state = data.get::<State>()
      .expect("Failed to get state");

    CreateApplicationCommand::default()
      .name(SNIPPET_NAME)
      .description("Shows a snippet")
      .create_option(|option| option
        .name("snippet")
        .description("The name of the snippet")
        .kind(CommandOptionType::String)
        .required(true)
        .gen_opts(state)
      )
      .to_owned()
  }

  async fn invoke(ctx: &Context, command: &ApplicationCommandInteraction) {
    let id = get_option_value(command, 0, "id");

    if let CommandDataOptionValue::String(id) = id {
      command.defer(&ctx.http).await.expect("Failed to defer interaction");

      let data = ctx.data.read().await;
      let state = data.get::<State>()
        .expect("Failed to get state");
      followup_snippet(ctx, state, command, id).await;
    }
  }
}

#[async_trait]
impl SlashCommand for SetSnippetCommand {
  async fn register(_: &Context) -> CreateApplicationCommand {
    CreateApplicationCommand::default()
      .name(SET_SNIPPET_NAME)
      .description("Set a snippet's contents")
      .create_option(|o| o
        .name("id")
        .description("The snippet ID")
        .kind(CommandOptionType::String)
        .required(true)
      )
      .create_option(|o| o
        .name("title")
        .description("The embed title")
        .kind(CommandOptionType::String)
        .required(true)
      )
      .create_option(|o| o
        .name("content")
        .description("The embed content")
        .kind(CommandOptionType::String)
        .required(true)
      )
      .to_owned()
  }

  async fn invoke(ctx: &Context, command: &ApplicationCommandInteraction) {
    let id = get_option_value(command, 0, "id");
    let title = get_option_value(command, 1, "title");
    let content = get_option_value(command, 2, "content");

    command.defer(&ctx.http).await.expect("Failed to defer interaction");

    match (id, title, content)
    {
      (
        CommandDataOptionValue::String(id),
        CommandDataOptionValue::String(title),
        CommandDataOptionValue::String(content),
      ) => {
        let mut data = ctx.data.write().await;
        let state = data.get_mut::<State>().expect("Failed to get state");
        let old_snippet = state.snippets.iter_mut()
          .find(|s| s.id == id);

        match old_snippet {
          Some(o) => {
            o.title = title;
            o.content = content;
          },
          None => state.snippets.push(Snippet {
            id: id.clone(),
            title,
            content
          })
        }

        state.write();

        if let Err(err) = crate::commands::register(ctx).await {
          command.create_followup_message(&ctx.http, |f| f
            .embed(|e| e
              .title("Error")
              .description(format!("{}", err))
              .colour(crate::commands::ERROR_COLOR)
            )
          ).await.expect("Failed to reply with error");
        } else {
          followup_snippet(ctx, state, command, id).await;
        }
      },
      _ => ()
    }
  }
}

#[async_trait]
impl SlashCommand for RemoveSnippetCommand {
  async fn register(ctx: &Context) -> CreateApplicationCommand {
    let data = ctx.data.read().await;
    let state = data.get::<State>().expect("Failed to get state");

    CreateApplicationCommand::default()
      .name(REMOVE_SNIPPET_NAME)
      .description("Removes a snippet")
      .create_option(|o| o
        .name("id")
        .description("The snippet's ID")
        .kind(CommandOptionType::String)
        .required(true)
        .gen_opts(state)
      )
      .to_owned()
  }

  async fn invoke(ctx: &Context, command: &ApplicationCommandInteraction) {
    let id = get_option_value(command, 0, "id");

    if let CommandDataOptionValue::String(id) = id {
      command.defer(&ctx.http).await.expect("Failed to defer interaction");

      let mut data = ctx.data.write().await;
      let state = data.get_mut::<State>().expect("Failed to get state");

      let index = state.snippets.iter().position(|s| s.id == id);

      match index.clone() {
        Some(i) => {
          state.snippets.remove(i);

          command.create_followup_message(&ctx.http, |r| r
            .embed(|e| e
              .title("Removed snippet")
              .description(format!("Successfully removed the '{}' snippet", id))
              .color(crate::commands::OK_COLOR)
            )
          ).await.expect("Failed to follow up interaction");
        },
        None => {
          command.create_followup_message(&ctx.http, |r| r
            .embed(|e| e
              .title("Error")
              .description(&format!("Failed to find a snippet '{}'", id))
              .color(crate::commands::ERROR_COLOR)
            )
          ).await.expect("Failed to follow up interaction");
        }
      };
    }
  }
}

#[async_trait]
impl SlashCommand for ExportSnippetCommand {
  async fn register(ctx: &Context) -> CreateApplicationCommand {
    let data = ctx.data.read().await;
    let state = data.get::<State>()
      .expect("Failed to get state");

    CreateApplicationCommand::default()
      .name(EXPORT_SNIPPET_NAME)
      .description("Exports a snippet for editing")
      .create_option(|o| o
        .name("id")
        .description("The snippet's ID")
        .kind(CommandOptionType::String)
        .required(true)
        .gen_opts(state)
      )
      .to_owned()
  }

  async fn invoke(ctx: &Context, command: &ApplicationCommandInteraction) {
    let id = get_option_value(command, 0, "id");

    if let CommandDataOptionValue::String(id) = id {
      command.defer(&ctx.http).await.expect("Failed to defer interaction");

      let data = ctx.data.read().await;
      let state = data.get::<State>().expect("Failed to get state");

      let snippet = state.snippets
        .iter()
        .find(|s| id.eq(&s.id));

      if let Some(s) = snippet {
        command.create_followup_message(&ctx.http, |f| f
          .content(format!("```\n{}\n```", s.content.replace(r#"\n"#, "\n")))
          .embed(|e| e
            .title(&s.title)
            .description(&s.content)
            .color(crate::commands::OK_COLOR)
          )
        ).await.expect("Failed to reply with snippet contents");
      } else {
        command.create_followup_message(&ctx.http, |f| f
          .embed(|e| e
            .title("Error")
            .description(format!("Failed to find snippet '{}'", id))
          )
        ).await.expect("Failed to reply with error embed");
      }
    }
  }
}
