use std::borrow::Cow;

use crate::{
    commands::{respond_embed, respond_err, respond_ok},
    structures::{Embeddable, Snippet},
    Context, Error,
};
use poise::serenity_prelude::{
    self as serenity, CreateAttachment, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

#[allow(clippy::unused_async)]
async fn autocomplete_snippet<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> serenity::CreateAutocompleteResponse<'a> {
    let snippet_list: Vec<_> = ctx
        .data()
        .state
        .read()
        .unwrap()
        .snippets
        .iter()
        .map(Snippet::format_output)
        .filter(|name| name.to_lowercase().contains(&partial.to_lowercase()))
        .map(serenity::AutocompleteChoice::from)
        .collect();

    serenity::CreateAutocompleteResponse::new().set_choices(snippet_list)
}

/// Show a snippet
///
/// Allows usage of both just the id and the formatted name (id: title)
#[poise::command(slash_command, prefix_command, guild_only, track_edits)]
pub async fn snippet(
    ctx: Context<'_>,
    #[rest]
    #[description = "The snippet's id"]
    #[autocomplete = "autocomplete_snippet"]
    id: String,
) -> Result<(), Error> {
    // Lazily get snippet because this is a prefix command too.
    if let Some(snippet) = get_snippet_lazy(&ctx, &id) {
        let embed = snippet.embed();

        respond_embed(&ctx, embed, false).await;
    } else {
        respond_err(
            &ctx,
            "Failed to find snippet",
            &format!("Failed to find the snippet '{id}'"),
        )
        .await;
    }

    Ok(())
}

/// Creates a snippet
#[poise::command(rename = "create-snippet", slash_command, guild_only)]
pub async fn create_snippet(
    ctx: Context<'_>,
    #[description = "The snippet's id"] id: String,
    #[description = "The snippet's title"] title: String,
    #[description = "The snippet's content"] content: String,
) -> Result<(), Error> {
    let snippet = Snippet {
        id: id.clone(),
        title,
        content: content.replace(r"\n", "\n"),
    };

    let mut embed = snippet.embed();
    embed = embed.colour(super::OK_COLOUR);

    let embed = {
        let data = ctx.data();
        let mut rwlock_guard = data.state.write().unwrap();

        if let Some(position) = rwlock_guard.snippets.iter().position(|s| s.id.eq(&id)) {
            rwlock_guard.snippets.remove(position);
        }

        println!("New snippet created '{}: {}'", snippet.id, snippet.title);

        rwlock_guard.snippets.push(snippet.clone());
        rwlock_guard.write();

        embed.clone()
    };

    respond_embed(&ctx, embed, false).await;

    Ok(())
}

/// Edits a snippet
#[poise::command(rename = "edit-snippet", slash_command, guild_only)]
pub async fn edit_snippet(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_snippet"]
    #[description = "The snippet's id"]
    id: String,
    #[description = "The snippet's title"] title: Option<String>,
    #[description = "The snippet's content"] content: Option<String>,
) -> Result<(), Error> {
    match get_snippet_lazy(&ctx, &id) {
        Some(mut snippet) => {
            if let Some(title) = title {
                snippet.title = title;
            }

            if let Some(content) = content {
                snippet.content = content.replace(r"\n", "\n");
            }

            {
                let data = ctx.data();
                let mut rwlock_guard = data.state.write().unwrap();
                rwlock_guard.snippets.push(snippet.clone());
                println!("Snippet edited '{}: {}'", snippet.title, snippet.content);
                rwlock_guard.write();
            }

            let embed = snippet.embed().colour(super::OK_COLOUR);
            respond_embed(&ctx, embed, false).await;
        }
        None => {
            let title = &"Failed to edit snippet";
            let content = &&format!("The snippet '{id}' does not exist");
            respond_err(&ctx, title, content).await;
        }
    };

    Ok(())
}

/// Removes a snippet
#[poise::command(rename = "remove-snippet", slash_command, guild_only)]
pub async fn remove_snippet(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_snippet"]
    #[description = "The snippet's id"]
    id: String,
) -> Result<(), Error> {
    match get_snippet_lazy(&ctx, &id) {
        Some(snippet) => {
            remove_snippet_confirm(&ctx, &snippet).await?;
        }
        None => {
            let title = &"Failed to remove snippet";
            let content = &&format!("The snippet '{id}' does not exist");
            respond_err(&ctx, title, content).await;
        }
    }

    Ok(())
}

/// Lists all snippets
#[poise::command(
    rename = "list-snippets",
    aliases("list-snippet", "snippets"),
    slash_command,
    prefix_command,
    guild_only,
    track_edits
)]
pub async fn list_snippets(ctx: Context<'_>) -> Result<(), Error> {
    let snippets = { ctx.data().state.read().unwrap().snippets.clone() };

    if snippets.is_empty() {
        respond_err(
            &ctx,
            "Cannot send list of snippets",
            "There are no snippets to list!",
        )
        .await;
        return Ok(());
    }

    let pages: Vec<Vec<(String, String, bool)>> = snippets
        .iter()
        .map(|snippet| (snippet.id.clone(), snippet.title.clone(), true))
        .collect::<Vec<(String, String, bool)>>()
        .chunks(25)
        .map(<[(String, String, bool)]>::to_vec)
        .collect();

    super::paginate_lists(ctx, &pages, "Snippets").await?;

    Ok(())
}

/// Exports a snippet for user editing.
///
/// Allows usage of both just the id and the formatted name (id: title)
#[poise::command(rename = "export-snippet", slash_command, prefix_command, guild_only)]
pub async fn export_snippet(
    ctx: Context<'_>,
    #[rest]
    #[autocomplete = "autocomplete_snippet"]
    #[description = "The snippet's id"]
    id: String,
) -> Result<(), Error> {
    match get_snippet_lazy(&ctx, &id) {
        Some(snippet) => {
            let attachment = CreateAttachment::bytes(
                snippet.content.replace('\n', r"\n").into_bytes(),
                "snippet.txt",
            );
            let message = poise::CreateReply::default()
                .attachment(attachment)
                .embed(snippet.embed());
            ctx.send(message).await?;
        }
        None => {
            let title = &"Failed to export snippet";
            let content = &&format!("The snippet '{id}' does not exist");
            respond_err(&ctx, title, content).await;
        }
    }

    Ok(())
}

impl Embeddable for Snippet {
    fn embed(&self) -> CreateEmbed<'_> {
        CreateEmbed::default()
            .title(&self.title)
            .description(&self.content)
            .colour(super::ACCENT_COLOUR)
            .clone()
    }
}

// Exact matches the snippet id and name.
fn _get_snippet(ctx: &Context<'_>, id: &str) -> Option<Snippet> {
    let data = ctx.data();
    let rwlock_guard = data.state.read().unwrap();

    rwlock_guard
        .snippets
        .iter()
        .find(|s| s.format_output().eq(id))
        .cloned()
}

// Matches the snippet by checking if its starts with the id and name.
fn get_snippet_lazy(ctx: &Context<'_>, id: &str) -> Option<Snippet> {
    let data = ctx.data();
    let rwlock_guard = data.state.read().unwrap();

    rwlock_guard
        .snippets
        .iter()
        .find(|s| s.format_output().starts_with(id))
        .cloned()
}

fn rm_snippet(ctx: &Context<'_>, snippet: &Snippet) {
    let data = ctx.data();
    let mut rwlock_guard = data.state.write().unwrap();

    let index = rwlock_guard
        .snippets
        .iter()
        .position(|s| s.id == snippet.id)
        .expect("Snippet was not found in vec");

    println!("Removing snippet '{}: {}'", snippet.id, snippet.title);
    rwlock_guard.snippets.remove(index);
    rwlock_guard.write();
}

async fn remove_snippet_confirm(ctx: &Context<'_>, snippet: &Snippet) -> Result<(), Error> {
    let snippet_embed = snippet.embed();

    let ctx_id = ctx.id();
    let delete_id = format!("{ctx_id}cancel");
    let cancel_id = format!("{ctx_id}delete");

    let components = serenity::CreateActionRow::Buttons(Cow::Owned(vec![
        serenity::CreateButton::new(&cancel_id).label("Cancel"),
        serenity::CreateButton::new(&delete_id)
            .label("Delete")
            .style(serenity::ButtonStyle::Danger),
    ]));

    let builder: poise::CreateReply = poise::CreateReply::default()
        .content(format!(
            "Are you sure you want to delete snippet `{}`?",
            snippet.id
        ))
        .ephemeral(true)
        .embed(snippet_embed)
        .components(vec![components]);

    ctx.send(builder).await?;

    while let Some(press) =
        serenity::ComponentInteractionCollector::new(ctx.serenity_context().shard.clone())
            .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
            .timeout(std::time::Duration::from_secs(60))
            .await
    {
        if press.data.custom_id == delete_id {
            handle_delete(ctx, snippet, press).await?;
        } else if press.data.custom_id == cancel_id {
            handle_cancel(ctx, press).await?;
        }
    }

    Ok(())
}

async fn handle_delete(
    ctx: &Context<'_>,
    snippet: &Snippet,
    interaction: serenity::ComponentInteraction,
) -> Result<(), Error> {
    rm_snippet(ctx, snippet);
    interaction
        .create_response(
            ctx.http(),
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .content("Deleted!")
                    .embeds(vec![])
                    .components(vec![]),
            ),
        )
        .await?;

    let title = format!("{} removed a snippet", ctx.author().tag());
    let content = &&format!("Removed snippet `{}`", snippet.format_output());
    respond_ok(ctx, &title, content).await;

    Ok(())
}

async fn handle_cancel(
    ctx: &Context<'_>,
    interaction: serenity::ComponentInteraction,
) -> Result<(), Error> {
    interaction
        .create_response(
            ctx.http(),
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .content("Aborted.")
                    .embeds(vec![])
                    .components(vec![]),
            ),
        )
        .await?;
    Ok(())
}
