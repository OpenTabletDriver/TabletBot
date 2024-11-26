use std::sync::OnceLock;

use crate::{
    commands::{respond_embed, respond_err, respond_ok},
    structures::RepositoryDetails,
    Context, Error,
};

use poise::serenity_prelude::{
    self as serenity, Colour, CreateEmbed, CreateEmbedFooter, EditMessage, Message,
};
use regex::Regex;

#[allow(clippy::unused_async)]
async fn autocomplete_key<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> serenity::CreateAutocompleteResponse<'a> {
    let snippet_list: Vec<_> = {
        ctx.data()
            .state
            .read()
            .unwrap()
            .issue_prefixes
            .iter()
            .map(|s| s.0.clone())
            .filter(|name| name.contains(partial))
            .map(serenity::AutocompleteChoice::from)
            .collect()
    };

    serenity::CreateAutocompleteResponse::new().set_choices(snippet_list)
}

/// Create an embed in the current channel.
#[allow(clippy::too_many_arguments)]
#[poise::command(slash_command, guild_only)]
pub async fn embed(
    ctx: Context<'_>,
    #[description = "The embed title"] title: Option<String>,
    #[description = "The embed description"] description: Option<String>,
    #[description = "The color of the embed in hexidecimal form. (ex: ff00ff)"] color: Option<
        String,
    >,
    #[description = "The embed url"] url: Option<String>,
    #[description = "The embed image"] image: Option<String>,
    #[description = "The embed footer text"] footer: Option<String>,
    #[description = "The embed thumbnail"] thumbnail: Option<String>,
) -> Result<(), Error> {
    let at_least_one_property_set = title.is_some()
        || description.is_some()
        || image.is_some()
        || thumbnail.is_some()
        || footer.is_some();

    if !at_least_one_property_set {
        respond_err(
            &ctx,
            "Failed to respond with embed",
            "Please provide at least one title, description, image, footer or thumbnail",
        )
        .await;
        return Ok(());
    }
    let mut embed = CreateEmbed::default();

    if let Some(url) = url {
        match url.parse::<reqwest::Url>() {
            Ok(_) => {
                if title.is_some() {
                    embed = embed.url(url);
                } else {
                    respond_err(
                        &ctx,
                        "Failed to respond with embed",
                        "To set a url, you must set a title.",
                    )
                    .await;
                    return Ok(());
                }
            }
            Err(e) => {
                let title = "Invalid url provided";
                let content = &format!("The url '{url}' is not a valid url: {e}");
                respond_err(&ctx, title, content).await;
                return Ok(());
            }
        }
    }

    if let Some(title) = title {
        embed = embed.title(title);
    }

    if let Some(description) = description {
        embed = embed.description(description.replace(r"\n", "\n"));
    }
    if let Some(image) = image {
        embed = embed.image(image);
    }

    if let Some(footer) = footer {
        embed = embed.footer(CreateEmbedFooter::new(footer));
    }

    if let Some(thumbnail) = thumbnail {
        embed = embed.thumbnail(thumbnail);
    }

    if let Some(color) = color {
        match hex::decode(color.to_ascii_lowercase().replace('#', "")) {
            Ok(hex_arr) => {
                embed = embed.color(Colour::from_rgb(hex_arr[0], hex_arr[1], hex_arr[2]));
            }
            Err(e) => {
                let title = "Invalid color provided";
                let content = &format!(
                    "The color '{}' is not a valid hexadecimal color: {}",
                    &color, e
                );
                respond_err(&ctx, title, content).await;
            }
        }
    }

    respond_embed(&ctx, embed, false).await;

    Ok(())
}

/// Edits an embed already sent by the embed command.
#[allow(clippy::too_many_arguments)]
#[poise::command(rename = "edit-embed", slash_command, guild_only)]
pub async fn edit_embed(
    ctx: Context<'_>,
    #[description = "The message to be edited."] message: Message,
    #[description = "The embed title"] title: Option<String>,
    #[description = "The embed description"] description: Option<String>,
    #[description = "The color of the embed in hexidecimal form. (ex: ff00ff)"] color: Option<
        String,
    >,
    #[description = "The embed url"] url: Option<String>,
    #[description = "The embed image"] image: Option<String>,
    #[description = "The embed footer text"] footer: Option<String>,
    #[description = "The embed thumbnail"] thumbnail: Option<String>,
) -> Result<(), Error> {
    let mut msg_clone = message.clone();
    if message.author.id != ctx.cache().current_user().id {
        respond_err(
            &ctx,
            "Cannot edit message!",
            "I am not the author of the specified message!",
        )
        .await;
    }

    if let Some(interaction) = message.interaction {
        if interaction.name == "embed" {
            // Embed for checking reasons.
            let embed = &message.embeds[0];
            let mut embedb = CreateEmbed::default();

            if let Some(title) = title {
                if title != "_" {
                    embedb = embedb.title(title);
                }
            } else if let Some(t) = &embed.title {
                embedb = embedb.title(t.as_str());
            }

            if let Some(description) = description {
                if description != "_" {
                    embedb = embedb.description(description);
                }
            } else if let Some(d) = &embed.description {
                embedb = embedb.description(d.as_str());
            }

            if let Some(color) = color {
                if color != "_" {
                    match hex::decode(color.to_ascii_lowercase().replace('#', "")) {
                        Ok(hex_arr) => {
                            embedb =
                                embedb.color(Colour::from_rgb(hex_arr[0], hex_arr[1], hex_arr[2]));
                        }
                        Err(e) => {
                            let title = "Invalid color provided";
                            let content = &format!(
                                "The color '{}' is not a valid hexadecimal color: {}",
                                &color, e
                            );
                            respond_err(&ctx, title, content).await;
                        }
                    }
                }
            } else if let Some(c) = &embed.colour {
                embedb = embedb.color(c.0);
            }

            if let Some(url) = url {
                if url != "_" {
                    embedb = embedb.url(url);
                }
            } else if let Some(u) = &embed.url {
                embedb = embedb.url(u.as_str());
            }

            if let Some(image) = image {
                if image != "_" {
                    embedb = embedb.image(image);
                }
            } else if let Some(i) = &embed.image {
                embedb = embedb.image(i.url.clone());
            }

            if let Some(footer) = footer {
                if footer != "_" {
                    embedb = embedb.footer(CreateEmbedFooter::new(footer));
                }
            } else if let Some(f) = &embed.footer {
                embedb = embedb.footer(CreateEmbedFooter::new(f.text.clone()));
            }

            if let Some(thumbnail) = thumbnail {
                if thumbnail != "_" {
                    embedb = embedb.thumbnail(thumbnail);
                }
            } else if let Some(t) = &embed.thumbnail {
                embedb = embedb.thumbnail(t.url.clone());
            }

            let builder = EditMessage::default().embed(embedb);

            match msg_clone.edit(ctx, builder).await {
                Ok(()) => {
                    respond_ok(
                        &ctx,
                        "Successfully edited embed",
                        "The message has been edited successfully!",
                    )
                    .await;
                }
                Err(error) => {
                    // Better error handling later.
                    respond_err(&ctx, "Error while handling message!", &format!("{error}")).await;
                }
            }
        } else {
            respond_err(
                &ctx,
                "Failure to edit embed",
                "This message was an interaction, but not an embed interaction!",
            )
            .await;
        }
    } else {
        respond_err(
            &ctx,
            "Failure to edit embed",
            "This message is not an interaction!",
        )
        .await;
    };

    Ok(())
}

/// Adds a repository token
#[poise::command(rename = "add-repository", slash_command, guild_only)]
pub async fn add_repo(
    ctx: Context<'_>,
    #[description = "The key to the repository in a lowercase alphabetic string"] key: String,
    #[description = "The owner of the repository."] owner: String,
    #[description = "The respository name."] repository: String,
) -> Result<(), Error> {
    let key_regex = get_key_regex();
    let repo_details_regex = get_repo_details_regex();
    if !key_regex.is_match(&key) {
        respond_err(
            &ctx,
            "Key parsing error",
            "The key can only lowercase ASCII letters, digits, and the characters ., -, and _.",
        )
        .await;
        return Ok(());
    }
    if !repo_details_regex.is_match(&owner) || !repo_details_regex.is_match(&repository) {
        respond_err(
            &ctx,
            "Repository details parsing error",
            "Your inputs for owner and repository name must be valid repository names.",
        )
        .await;
        return Ok(());
    }

    {
        let data = ctx.data();
        let mut rwlock_guard = data.state.write().unwrap();

        let details = RepositoryDetails {
            owner: owner.clone(),
            name: repository.clone(),
        };

        rwlock_guard
            .issue_prefixes
            .insert(key.clone().to_lowercase(), details);

        println!(
            "Successfully added repository {} for **{}/{}**",
            key.to_lowercase(),
            owner,
            repository
        );
        rwlock_guard.write();
    };

    respond_ok(
        &ctx,
        "Successfully added issue token",
        &format!("{key}: {owner}/{repository}"),
    )
    .await;

    Ok(())
}

/// Removes a repository
#[poise::command(rename = "remove-repository", slash_command, guild_only)]
pub async fn remove_repo(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_key"]
    #[description = "The repository key."]
    key: String,
) -> Result<(), Error> {
    // I know we could just do rm_repo, but that doesn't return a result.
    // I may change this in the future, but before I do that I'll probably
    // impl a solution directly into the types?

    // not sure why I have to do this, it won't settle otherwise.
    let key_str = format!("The repository with the key '{key}' has been removed");
    match get_repo_details(&ctx, &key) {
        Some(_) => {
            rm_repo(&ctx, &key);

            respond_ok(&ctx, "Successfully removed repository!", &key_str).await;
        }
        None => {
            let title = "Failure to find repository";
            let content = format!("The key '{key}' does not exist.");
            respond_err(&ctx, title, &content).await;
        }
    };

    Ok(())
}

/// Lists all repositories
#[poise::command(
    rename = "list-repositories",
    aliases("repos-list", "list-repos", "repos"),
    slash_command,
    prefix_command,
    guild_only,
    track_edits
)]
pub async fn list_repos(ctx: Context<'_>) -> Result<(), Error> {
    let tokens = { ctx.data().state.read().unwrap().issue_prefixes.clone() };

    if tokens.is_empty() {
        respond_err(
            &ctx,
            "Cannot send list of repositories",
            "There are no repositories to list!",
        )
        .await;
        return Ok(());
    }

    let pages: Vec<Vec<(String, String, bool)>> = tokens
        .iter()
        .map(|token| {
            (
                token.0.clone(),
                format!("{}/{}", token.1.owner, token.1.name),
                true,
            )
        })
        .collect::<Vec<(String, String, bool)>>()
        .chunks(25)
        .map(<[(String, String, bool)]>::to_vec)
        .collect();

    super::paginate_lists(ctx, &pages, "Repositories").await?;

    Ok(())
}

fn get_repo_details(ctx: &Context<'_>, key: &str) -> Option<RepositoryDetails> {
    let data = ctx.data();
    let rwlock_guard = data.state.read().unwrap();

    rwlock_guard.issue_prefixes.get(key).cloned()
}

fn rm_repo(ctx: &Context<'_>, key: &str) {
    let data = ctx.data();
    let mut rwlock_guard = data.state.write().unwrap();

    rwlock_guard.issue_prefixes.remove(key);
}

fn get_key_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"[a-z+]+$").unwrap())
}

fn get_repo_details_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^([a-zA-Z0-9-_.]+)*$").unwrap())
}
