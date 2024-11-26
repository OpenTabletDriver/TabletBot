use std::{borrow::Cow, sync::OnceLock, time::Duration};

use crate::{commands::interaction_err, structures::Embeddable, Data, FrameworkContext};

use aformat::aformat;
use octocrab::models::issues::Issue;
use poise::serenity_prelude::{
    self as serenity, ButtonStyle, CreateActionRow, CreateButton, CreateEmbed,
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

pub async fn message(framework: FrameworkContext<'_>, message: &Message) {
    let data = framework.user_data();
    let ctx = framework.serenity_context;

    let issues = issues(&data, message).await;

    let Some(issues) = issues else { return };

    let embeds = issues.iter().map(|i| i.embed()).collect::<Vec<_>>();

    let typing = message.channel_id.start_typing(ctx.http.clone());

    let ctx_id = message.id.get().to_arraystring();

    let remove_id = aformat!("{ctx_id}delete");
    let hide_body_id = aformat!("{ctx_id}hide_body");

    let remove = CreateActionRow::Buttons(Cow::Owned(vec![CreateButton::new(&*remove_id)
        .label("delete")
        .style(ButtonStyle::Danger)]));

    let components = serenity::CreateActionRow::Buttons(Cow::Owned(vec![
        CreateButton::new(&*remove_id)
            .label("delete")
            .style(ButtonStyle::Danger),
        CreateButton::new(&*hide_body_id).label("hide body"),
    ]));

    let content: serenity::CreateMessage = serenity::CreateMessage::default()
        .embeds(embeds)
        .reference_message(message)
        .components(vec![components]);
    let msg_result = message.channel_id.send_message(&ctx.http, content).await;
    typing.stop();

    let mut msg_deleted = false;
    let mut body_hid = false;
    while let Some(press) = serenity::ComponentInteractionCollector::new(ctx.shard.clone())
        .filter(move |press| press.data.custom_id.starts_with(&*ctx_id))
        .timeout(Duration::from_secs(60))
        .await
    {
        let has_perms = press.member.as_ref().is_some_and(|member| {
            member
                .permissions
                .is_some_and(|member_perms| member_perms.contains(Permissions::MANAGE_MESSAGES))
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
                    .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
                    .await;
                if let Ok(ref msg) = msg_result {
                    let _ = msg.delete(&ctx.http, None).await;
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
                            &ctx.http,
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

async fn issues(data: &Data, message: &Message) -> Option<Vec<Issue>> {
    let mut collection = vec![];
    let client = octocrab::instance();
    let ratelimit = client.ratelimit();

    let regex = get_issue_regex();

    let custom_repos = { data.state.read().unwrap().issue_prefixes.clone() };

    let issues = client.issues(DEFAULT_REPO_OWNER, DEFAULT_REPO_NAME);

    for capture in regex.captures_iter(&message.content) {
        let Some(m) = capture.get(2) else { continue };

        let issue_num = m.as_str().parse::<u64>().expect("Match is not a number");

        let ratelimit = ratelimit
            .get()
            .await
            .expect("Failed to get github rate limit");

        if let Some(repo) = capture.get(1) {
            let repository = custom_repos.get(&repo.as_str().to_lowercase());
            // if there is not a repository that matches the input, ignore.
            let Some(repository) = repository else {
                continue;
            };

            let (owner, repo) = repository.get();

            if let Ok(issue) = client.issues(owner, repo).get(issue_num).await {
                collection.push(issue);
                continue;
            }
        }

        if ratelimit.rate.remaining >= 1 {
            if let Ok(issue) = issues.get(issue_num).await {
                collection.push(issue);
            }
        }
    }

    if collection.is_empty() {
        None
    } else {
        Some(collection)
    }
}

fn get_issue_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r" ?([a-zA-Z0-9-_.]+)?#([0-9]+) ?").unwrap())
}
