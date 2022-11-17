use serenity::async_trait;
use serenity::builder::{CreateApplicationCommand, CreateApplicationCommandOption};
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue};
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::model::prelude::ReactionType;
use serenity::prelude::Context;
use crate::commands::{get_option_value, SlashCommand};
use crate::structures::{ReactionRole, State};


pub const ADD_REACTION_ROLE_NAME: &str = "add-reaction-role";
pub struct AddReactionRoleCommand;

pub const REMOVE_REACTION_ROLE_NAME: &str = "remove-reaction-role";
pub struct RemoveReactionRoleCommand;

async fn defer_ephemeral(ctx: &Context, command: &ApplicationCommandInteraction) {
  command.create_interaction_response(ctx, |r| r
    .kind(InteractionResponseType::DeferredChannelMessageWithSource)
    .interaction_response_data(|d| d.ephemeral(true))
  ).await.expect("Failed to defer interaction");
}

#[async_trait]
impl SlashCommand for AddReactionRoleCommand {
  async fn register(_: &Context) -> CreateApplicationCommand {
    CreateApplicationCommand::default()
      .name(ADD_REACTION_ROLE_NAME)
      .description("Adds a reaction role")
      .create_option(|o| o
        .name("emote")
        .description("The emote to use for the reaction role")
        .kind(CommandOptionType::String)
        .required(true)
      )
      .create_option(|o| o
        .name("target")
        .description("The target message id")
        .kind(CommandOptionType::String)
        .required(true)
      )
      .create_option(|o| o
        .name("role")
        .description("The role to assign when reacted with this emote")
        .kind(CommandOptionType::Role)
        .required(true)
      )
      .to_owned()
  }

  async fn invoke(ctx: &Context, command: &ApplicationCommandInteraction) {
    let emote = get_option_value(command, 0, "emote");
    let target = get_option_value(command, 1, "target");
    let role = get_option_value(command, 2, "role");

    defer_ephemeral(ctx, command).await;

    match (emote, target, role) {
      (
        CommandDataOptionValue::String(emote),
        CommandDataOptionValue::String(target),
        CommandDataOptionValue::Role(role)
      ) => {
        let message_id = target.parse::<u64>().expect("Target message id is not u64");
        let role_id = *role.id.as_u64();

        let message = command.channel_id.message(ctx, message_id).await
          .expect("Failed to get target message");

        let reaction_role = ReactionRole {
          message_id,
          role_id,
          emote: emote.to_owned()
        };

        let mut data = ctx.data.write().await;
        let state = data.get_mut::<State>().expect("Failed to get state");

        state.reaction_roles.push(reaction_role);
        state.write();

        message.delete_reaction_emoji(ctx, ReactionType::Unicode(emote.clone())).await
          .expect("Failed to delete reaction");

        if let Err(err) = crate::commands::register(ctx).await {
          command.create_followup_message(&ctx.http, |f| f
            .embed(|e| e
              .title("Error")
              .description(format!("{}", err))
              .colour(crate::commands::ERROR_COLOR)
            )
          ).await.expect("Failed to reply with error");
        } else {
          command.create_followup_message(ctx, |r| r
            .ephemeral(true)
            .embed(|e| e
              .title("Reaction role")
              .description("Successfully added a reaction role")
              .color(crate::commands::OK_COLOR)
              .field("Emote", emote, false)
              .field("Message ID", target, false)
              .field("Role", role.id.as_u64(), false)
            )
          ).await.expect("Failed to respond to interaction");
        }
      },
      _ => panic!("Invalid arguments to interaction")
    }
  }
}

#[async_trait]
impl SlashCommand for RemoveReactionRoleCommand {
  async fn register(ctx: &Context) -> CreateApplicationCommand {
    let data = ctx.data.read().await;
    let state = data.get::<State>()
      .expect("Failed to get state");

    CreateApplicationCommand::default()
      .name(REMOVE_REACTION_ROLE_NAME)
      .description("Removes a reaction role")
      .create_option(|o| o
        .name("target")
        .description("The target message id")
        .kind(CommandOptionType::String)
        .required(true)
        .gen_opts(state)
      )
      .to_owned()
  }

  async fn invoke(ctx: &Context, command: &ApplicationCommandInteraction) {
    defer_ephemeral(ctx, command).await;

    if let CommandDataOptionValue::String(target) = get_option_value(command, 0, "target message") {
      let message_id = target.parse::<u64>().expect("Target message id is not u64");

      let mut data = ctx.data.write().await;
      let state = data.get_mut::<State>().expect("Failed to get state");

      let index = state.reaction_roles.iter()
        .position(|r| r.message_id == message_id);

      if let Some(index) = index {
        let react_role = state.reaction_roles.get(index)
          .expect("Failed to get reaction role at existing index")
          .clone();

        state.reaction_roles.remove(index);
        state.write();

        command.create_followup_message(ctx, |r| r
          .ephemeral(true)
          .embed(|e| e
            .title("Reaction role")
            .description("Succesfully removed reaction role")
            .field("Message ID", react_role.message_id, false)
            .field("Emote", react_role.emote, false)
            .field("Role", react_role.role_id, false)
          )
        ).await.expect("Failed to respond to interaction");
      }
    } else {
      panic!("Invalid arguments to interaction")
    }
  }
}

trait ReactionOpts {
  fn gen_opts(&mut self, state: &State) -> &mut Self;
}

impl ReactionOpts for CreateApplicationCommandOption {
  fn gen_opts(&mut self, state: &State) -> &mut Self {
    for react_role in &state.reaction_roles {
      let name = format!("{}", react_role.message_id);
      self.add_string_choice(name, react_role.message_id);
    }

    self
  }
}
