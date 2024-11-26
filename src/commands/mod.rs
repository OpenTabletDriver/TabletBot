pub mod snippets;
pub mod udev;
pub mod utils;

pub(crate) const ACCENT_COLOUR: Colour = Colour(0x8957e5);
pub(crate) const OK_COLOUR: Colour = Colour(0x2ecc71);
pub(crate) const ERROR_COLOUR: Colour = Colour(0xe74c3c);

use std::borrow::Cow;

use to_arraystring::ToArrayString;

use crate::{Context, Error};

use poise::serenity_prelude::{
    self as serenity, Colour, ComponentInteraction, ComponentInteractionCollector, CreateActionRow,
    CreateButton, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use poise::CreateReply;

#[poise::command(prefix_command, hide_in_help)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;

    Ok(())
}

pub async fn respond_embed(ctx: &Context<'_>, embed: CreateEmbed<'_>, ephemeral: bool) {
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

pub async fn interaction_err(ctx: &serenity::Context, press: &ComponentInteraction, content: &str) {
    let builder = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .embed(
                CreateEmbed::new()
                    .title("Unable to execute interaction")
                    .description(content)
                    .colour(ERROR_COLOUR),
            )
            .ephemeral(true),
    );
    let _ = press.create_response(&ctx.http, builder).await;
}

enum Kind {
    Next,
    Previous,
}

impl Kind {
    fn from_id(id: &str, ctx_id: &str) -> Option<Self> {
        let this = match id.strip_prefix(ctx_id)? {
            "next" => Self::Next,
            "prev" => Self::Previous,
            _ => return None,
        };

        Some(this)
    }
}

pub async fn paginate_lists(
    ctx: crate::Context<'_>,
    pages: &[Vec<(String, String, bool)>],
    embed_title: &str,
) -> Result<(), Error> {
    let ctx_id = ctx.id().to_arraystring();

    let prev_button_id = format!("{ctx_id}prev");
    let next_button_id = format!("{ctx_id}next");

    let colour = Colour::TEAL;

    let components = CreateActionRow::Buttons(Cow::Owned(vec![
        CreateButton::new(&*prev_button_id).emoji('◀'),
        CreateButton::new(&*next_button_id).emoji('▶'),
    ]));
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
        while let Some(press) =
            ComponentInteractionCollector::new(ctx.serenity_context().shard.clone())
                .filter(move |press| press.data.custom_id.starts_with(ctx_id.as_str()))
                .timeout(std::time::Duration::from_secs(180))
                .await
        {
            match Kind::from_id(&press.data.custom_id, &ctx_id) {
                Some(Kind::Next) => {
                    current_page += 1;
                    if current_page >= pages.len() {
                        current_page = 0;
                    }
                }
                Some(Kind::Previous) => {
                    current_page = current_page.checked_sub(1).unwrap_or(pages.len() - 1);
                }
                None => continue,
            }

            press
                .create_response(
                    ctx.http(),
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
