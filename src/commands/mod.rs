pub mod snippets;
pub mod utils;

pub(crate) const ACCENT_COLOUR: Colour = Colour(0x8957e5);
pub(crate) const OK_COLOUR: Colour = Colour(0x2ecc71);
pub(crate) const ERROR_COLOUR: Colour = Colour(0xe74c3c);

use crate::{Context, Error};

use poise::serenity_prelude::{
    self as serenity, Colour, ComponentInteractionCollector, CreateActionRow, CreateButton,
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use poise::CreateReply;

#[poise::command(prefix_command, hide_in_help)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;

    Ok(())
}

pub async fn respond_embed(ctx: &Context<'_>, embed: CreateEmbed, ephemeral: bool) {
    let builder = poise::CreateReply::default()
        .embed(embed)
        .ephemeral(ephemeral);
    let result = ctx.send(builder).await;

    if let Err(e) = result {
        println!("Failed to respond: {e}");
    }
}

pub async fn respond_ok(ctx: &Context<'_>, title: &str, content: &str) {
    let embed = CreateEmbed::default()
        .title(title)
        .description(content)
        .colour(OK_COLOUR);

    respond_embed(ctx, embed, false).await;
}

pub async fn respond_err(ctx: &Context<'_>, title: &str, content: &str) {
    let embed = CreateEmbed::default()
        .title(title)
        .description(content)
        .colour(ERROR_COLOUR);

    respond_embed(ctx, embed, false).await;
}

pub async fn paginate_lists<U, E>(
    ctx: poise::Context<'_, U, E>,
    pages: &[Vec<(String, String, bool)>],
    embed_title: &str,
) -> Result<(), Error> {
    let ctx_id = ctx.id();
    let prev_button_id = format!("{ctx_id}prev");
    let next_button_id = format!("{ctx_id}next");

    let colour = Colour::TEAL;

    let components = CreateActionRow::Buttons(vec![
        CreateButton::new(&prev_button_id).emoji('◀'),
        CreateButton::new(&next_button_id).emoji('▶'),
    ]);
    let mut current_page = 0;

    // Don't paginate if its one page.
    let reply = if pages.len() > 1 {
        CreateReply::default()
            .embed(
                CreateEmbed::default()
                    .title(embed_title)
                    .fields(pages[current_page].clone())
                    .colour(colour)
                    .footer(CreateEmbedFooter::new(format!(
                        "Page: {}/{}",
                        current_page + 1,
                        pages.len()
                    ))),
            )
            .components(vec![components])
    } else {
        CreateReply::default().embed(
            CreateEmbed::default()
                .title(embed_title)
                .colour(colour)
                .fields(pages[current_page].clone()),
        )
    };

    let msg = ctx.send(reply).await?;

    if pages.len() > 1 {
        while let Some(press) = ComponentInteractionCollector::new(ctx)
            .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
            .timeout(std::time::Duration::from_secs(180))
            .await
        {
            if press.data.custom_id == next_button_id {
                current_page += 1;
                if current_page >= pages.len() {
                    current_page = 0;
                }
            } else if press.data.custom_id == prev_button_id {
                current_page = current_page.checked_sub(1).unwrap_or(pages.len() - 1);
            } else {
                continue;
            }

            press
                .create_response(
                    ctx.serenity_context(),
                    CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::new().embed(
                            serenity::CreateEmbed::new()
                                .title(embed_title)
                                .colour(colour)
                                .fields(pages[current_page].clone())
                                .footer(CreateEmbedFooter::new(format!(
                                    "Page: {}/{}",
                                    current_page + 1,
                                    pages.len()
                                ))),
                        ),
                    ),
                )
                .await?;
        }
        // Remove components after timeout.
        msg.edit(
            ctx,
            poise::CreateReply::default()
                .embed(
                    serenity::CreateEmbed::default()
                        .title(embed_title)
                        .colour(colour)
                        .fields(pages[current_page].clone())
                        .footer(CreateEmbedFooter::new(format!(
                            "Page: {}/{}",
                            current_page + 1,
                            pages.len()
                        ))),
                )
                .components(vec![]),
        )
        .await?;
    }

    Ok(())
}
