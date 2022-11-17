use serenity::model::prelude::{Reaction, RoleId};
use serenity::prelude::Context;
use crate::structures::State;

pub async fn reaction_add(ctx: &Context, reaction: &Reaction) {
  if let Some(role) = get_reaction_role(ctx, reaction).await {
    let user = reaction.user(ctx).await
      .expect("Reaction not performed by a user");

    let guild = reaction.guild_id
      .expect("Failed to get guild");

    let mut member = guild.member(ctx, &user).await
      .expect(&format!("Failed to get member '{}'", &user.id));

    if let Err(e) = member.add_role(ctx, role).await {
      println!("Failed to add role to user '{}': {}", member, e)
    }
  }
}

pub async fn reaction_remove(ctx: &Context, reaction: &Reaction) {
  if let Some(role) = get_reaction_role(ctx, reaction).await {
    let user = reaction.user(ctx).await
      .expect("Reaction not performed by a user");

    let guild = reaction.guild_id
      .expect("Failed to get guild");

    let mut member = guild.member(ctx, &user).await
      .expect(&format!("Failed to get member roles for user '{}'", &user.id));

    if let Err(e) = member.remove_role(ctx, role).await {
      println!("Failed to remove role from user '{}': {}", member, e)
    }
  }
}

async fn get_reaction_role<'a>(ctx: &'a Context, reaction: &Reaction) -> Option<RoleId> {
  let data = ctx.data.read().await;
  let state = data.get::<State>().expect("Failed to get state");
  let reaction_role = state.reaction_roles.iter()
    .find(|r| r.message_id == reaction.message_id.0);

  match (reaction_role, reaction.guild_id) {
    (
      Some(reaction_role),
      Some(guild)
    ) => {
      let roles = guild.roles(ctx).await
        .expect("Failed to get roles in guild");

      let role = roles
        .iter()
        .find(|r| r.0.0 == reaction_role.role_id);

      if let Some(role) = role {
        Some(*role.0)
      } else {
        None
      }
    },
    _ => None
  }
}
