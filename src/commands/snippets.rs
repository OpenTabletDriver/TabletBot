use crate::{
    commands::{respond_embed, respond_err, respond_ok},
    structures::{Embeddable, Snippet},
    Context, Error,
};
use ::serenity::futures::{Stream, StreamExt};
use poise::serenity_prelude::{futures, Colour, CreateAttachment, CreateEmbed};

async fn autocomplete_snippet<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a {
    let snippet_list: Vec<String> = {
        ctx.data()
            .state
            .read()
            .unwrap()
            .snippets
            .iter()
            .take(25)
            .map(|s| format!("{}: {}", s.id, s.title))
            .collect()
    };

    futures::stream::iter(snippet_list)
        .filter(move |name| futures::future::ready(name.starts_with(partial)))
        .map(|name| name.to_string())
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
    if let Some(snippet) = get_snippet_lazy(&ctx, &id).await {
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
    // I really don't like the code I wrote here.
    let embed = {
        let mut rwlock_guard = ctx.data().state.write().unwrap();

        if let Some(position) = rwlock_guard.snippets.iter().position(|s| s.id.eq(&id)) {
            rwlock_guard.snippets.remove(position);
        }

        let snippet = Snippet {
            id: id.clone(),
            title: title.clone(),
            content: content.replace(r"\n", "\n"),
        };

        rwlock_guard.snippets.push(snippet.clone());

        rwlock_guard.snippets = rwlock_guard.snippets.clone();
        println!("New snippet created '{}: {}'", id, title);
        rwlock_guard.write();

        let mut embed = snippet.embed();
        embed = embed.colour(super::OK_COLOUR);

        if rwlock_guard.snippets.len() > 25 {
            embed = embed.field(
                "Warning",
                "There are more than 25 snippets, some may not appear in the snippet list.",
                false,
            );
        }

        embed
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
    match get_snippet(&ctx, &id).await {
        Some(mut snippet) => {
            if let Some(title) = title {
                snippet.title = title;
            }

            if let Some(content) = content {
                snippet.content = content.replace(r"\n", "\n");
            }

            {
                let mut rwlock_guard = ctx.data().state.write().unwrap();
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
            respond_err(&ctx, title, content).await
        }
    };

    Ok(())
}

/// Delete snippet
///
/// Must use the full formatted snippet name (id: title)
#[poise::command(rename = "delete-snippet", slash_command, guild_only)]
pub async fn delete_snippet(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_snippet"]
    #[description = "The snippet's id"]
    id: String,
) -> Result<(), Error> {
    match get_snippet(&ctx, &id).await {
        Some(snippet) => {
            rm_snippet(&ctx, &snippet).await;
            let title = &"Snippet successfully removed";
            let content = &&format!("Removed snippet '{}: {}'", snippet.id, snippet.title);
            respond_ok(&ctx, title, content).await;
        }
        None => {
            let title = &"Failed to remove snippet";
            let content = &&format!("The snippet '{id}' does not exist");
            respond_err(&ctx, title, content).await
        }
    }

    Ok(())
}

/// Lists all snippets
#[poise::command(
    rename = "list-snippets",
    slash_command,
    prefix_command,
    guild_only,
    track_edits
)]
pub async fn list_snippets(ctx: Context<'_>) -> Result<(), Error> {
    let snippets = { ctx.data().state.read().unwrap().snippets.clone() };

    let mut embed = CreateEmbed::default().title("Snippets").color(Colour::TEAL);

    // fields are limited to 25 max, we can't display more than 25 snippets in the snippets command
    // due to a discord limitation.
    for snippet in snippets.iter().take(25) {
        embed = embed.field(format!("`{}`", snippet.id), &snippet.title, false);
    }

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

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
    match get_snippet_lazy(&ctx, &id).await {
        Some(snippet) => {
            let attachment = CreateAttachment::bytes(
                format!("{}", &snippet.content.replace('\n', r"\n")),
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
            respond_err(&ctx, title, content).await
        }
    }

    Ok(())
}

impl Embeddable for Snippet {
    fn embed(&self) -> CreateEmbed {
        CreateEmbed::default()
            .title(&self.title)
            .description(&self.content)
            .colour(super::ACCENT_COLOUR)
            .clone()
    }
}

// Exact matches the snippet id and name.
async fn get_snippet(ctx: &Context<'_>, id: &str) -> Option<Snippet> {
    let data = ctx.data();
    let rwlock_guard = data.state.read().unwrap();

    rwlock_guard
        .snippets
        .iter()
        .find(|s| s.format_output().eq(id))
        .cloned()
}

// Matches the snippet by checking if its starts with the id and name.
async fn get_snippet_lazy(ctx: &Context<'_>, id: &str) -> Option<Snippet> {
    let data = ctx.data();
    let rwlock_guard = data.state.read().unwrap();

    rwlock_guard
        .snippets
        .iter()
        .find(|s| s.format_output().starts_with(id))
        .cloned()
}

async fn rm_snippet(ctx: &Context<'_>, snippet: &Snippet) {
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
