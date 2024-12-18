use std::{sync::OnceLock, time::Duration};

use crate::{commands::interaction_err, structures::Embeddable, Data};

use arrayvec::ArrayString;
use poise::serenity_prelude::{
    self as serenity, ButtonStyle, Context, CreateActionRow, CreateButton, CreateEmbed,
    CreateInteractionResponse, Message, Permissions,
};
use regex::Regex;
use to_arraystring::ToArrayString;

const DEFAULT_REPO_OWNER: &str = "OpenTabletDriver";
const DEFAULT_REPO_NAME: &str = "OpenTabletDriver";

mod utils;

enum Kind {
    Delete,
    HideBody,
}

impl Kind {
    fn from_id(id: &str, ctx_id: &str) -> Option<Self> {
        let this = match id.strip_prefix(ctx_id)? {
            "delete" => Self::Delete,
            "hide_body" => Self::HideBody,
            _ => return None,
        };

        Some(this)
    }
}

pub async fn message(data: &Data, ctx: &Context, message: &Message) {
    if let Some(embeds) = issue_embeds(data, message).await {
        let typing = message.channel_id.start_typing(&ctx.http);

        // usually I would use poise context to generate a unique id, but its not available
        // on events, but we also aren't handling different invocation of this on a single message,
        // so its not actually needed.

        // The max length of this is known (its just a u64) and with a little neat library
        // we can avoid even a stack allocation! (thanks gnome)
        let ctx_id = message.id.get().to_arraystring();

        // we know the max size so we don't need to allocate.
        // 20 + 6
        let mut remove_id = ArrayString::<26>::new();
        remove_id.push_str(&ctx_id);
        remove_id.push_str("delete");

        // 20 + 9
        let mut hide_body_id = ArrayString::<29>::new();
        hide_body_id.push_str(&ctx_id);
        hide_body_id.push_str("hide_body");

        let remove = CreateActionRow::Buttons(vec![CreateButton::new(&*remove_id)
            .label("delete")
            .style(ButtonStyle::Danger)]);

        let components = serenity::CreateActionRow::Buttons(vec![
            CreateButton::new(&*remove_id)
                .label("delete")
                .style(ButtonStyle::Danger),
            CreateButton::new(&*hide_body_id).label("hide body"),
        ]);

        let content: serenity::CreateMessage = serenity::CreateMessage::default()
            .embeds(embeds)
            .reference_message(message)
            .components(vec![components]);
        let msg_result = message.channel_id.send_message(ctx, content).await;
        typing.stop();

        let mut msg_deleted = false;
        let mut body_hid = false;
        while let Some(press) = serenity::ComponentInteractionCollector::new(ctx)
            .filter(move |press| press.data.custom_id.starts_with(&*ctx_id))
            .timeout(Duration::from_secs(60))
            .await
        {
            let has_perms = press.member.as_ref().map_or(false, |member| {
                member.permissions.map_or(false, |member_perms| {
                    member_perms.contains(Permissions::MANAGE_MESSAGES)
                })
            });

            // Users who do not own the message or have permissions cannot execute the interactions.
            if !(press.user.id == message.author.id || has_perms) {
                interaction_err(
                    ctx,
                    &press,
                    "Unable to use interaction because you are missing `MANAGE_MESSAGES`.",
                )
                .await;

                continue;
            }

            match Kind::from_id(&press.data.custom_id, &ctx_id) {
                Some(Kind::Delete) => {
                    let _ = press
                        .create_response(ctx, CreateInteractionResponse::Acknowledge)
                        .await;
                    if let Ok(ref msg) = msg_result {
                        let _ = msg.delete(ctx).await;
                    }
                    msg_deleted = true;
                }
                Some(Kind::HideBody) => {
                    if !body_hid {
                        let mut hid_body_embeds: Vec<CreateEmbed> = Vec::new();
                        if let Ok(ref msg) = msg_result {
                            for mut embed in msg.embeds.clone() {
                                embed.description = None;
                                let embed: CreateEmbed = embed.clone().into();
                                hid_body_embeds.push(embed);
                            }
                        }

                        let _ = press
                            .create_response(
                                ctx,
                                serenity::CreateInteractionResponse::UpdateMessage(
                                    serenity::CreateInteractionResponseMessage::new()
                                        .embeds(hid_body_embeds)
                                        .components(vec![remove.clone()]),
                                ),
                            )
                            .await;
                    }
                    body_hid = true;
                }
                None => {}
            }
        }

        // Triggers on timeout.
        if !msg_deleted {
            if let Ok(mut msg) = msg_result {
                let _ = msg
                    .edit(ctx, serenity::EditMessage::default().components(vec![]))
                    .await;
            }
        }
    }
}

async fn issue_embeds(data: &Data, message: &Message) -> Option<Vec<CreateEmbed>> {
    let mut embeds: Vec<CreateEmbed> = vec![];
    let client = octocrab::instance();
    let ratelimit = client.ratelimit();

    let regex = get_issue_regex();

    let custom_repos = { data.state.read().unwrap().issue_prefixes.clone() };

    let mut issues = client.issues(DEFAULT_REPO_OWNER, DEFAULT_REPO_NAME);
    let mut prs = client.pulls(DEFAULT_REPO_OWNER, DEFAULT_REPO_NAME);

    for capture in regex.captures_iter(&message.content) {
        if let Some(m) = capture.get(2) {
            let issue_num = m.as_str().parse::<u64>().expect("Match is not a number");

            if let Some(repo) = capture.get(1) {
                let repository = custom_repos.get(&repo.as_str().to_lowercase());
                if let Some(repository) = repository {
                    let (owner, repo) = repository.get();

                    issues = client.issues(owner, repo);
                    prs = client.pulls(owner, repo);
                } else {
                    continue; // discards when it doesn't match a repo.
                };
            }

            let ratelimit = ratelimit
                .get()
                .await
                .expect("Failed to get github rate limit");

            if ratelimit.rate.remaining > 2 {
                if let Ok(pr) = prs.get(issue_num).await {
                    embeds.push(pr.embed());
                } else if let Ok(issue) = issues.get(issue_num).await {
                    embeds.push(issue.embed());
                }
            }
        }
    }

    if embeds.is_empty() {
        None
    } else {
        Some(embeds)
    }
}

fn get_issue_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r" ?([a-zA-Z0-9-_.]+)?#([0-9]+)\b").unwrap())
}
