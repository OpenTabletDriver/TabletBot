use lazy_static::lazy_static;
use octocrab::models::issues::Issue;
use octocrab::models::pulls::PullRequest;
use regex::Regex;
use serenity::builder::CreateEmbed;
use serenity::model::prelude::Message;
use serenity::prelude::Context;
use serenity::utils::Colour;

const OPEN_COLOUR: Colour = Colour(0x238636);
const RESOLVED_COLOUR: Colour = Colour(0x8957e5);
const CLOSED_COLOUR: Colour = Colour(0xda3633);

lazy_static! {
  // TODO: Fix overlapping: '#1 #2 #3' will only match #1 and #3
  // look-around, including look-ahead and look-behind, is not supported
  static ref ISSUE_REGEX: Regex = Regex::new(r#"(?:^|\s)(?:(?P<owner>\w+)/)?(?P<repo>\w+)?#(?P<num>\d+)(?:$|\s)"#)
    .expect("Expected numbers regex");
}

pub async fn message(ctx: &Context, message: &Message) {
  if let Some(embeds) = issue_embeds(message).await {
    let result = message.channel_id.send_message(&ctx.http, |f| f
      .reference_message(message)
      .set_embeds(embeds)
    ).await;

    match result {
      Err(e) => panic!("Failed to reply with issue embeds {:#?}", e),
      _ => ()
    }
  }
}

async fn issue_embeds(message: &Message) -> Option<Vec<CreateEmbed>> {
  let mut captures = ISSUE_REGEX.captures_iter(&message.content)
    .peekable();

  if captures.peek().is_some() {
    let mut embeds: Vec<CreateEmbed> = vec![];
    let client = octocrab::instance();
    let ratelimit = client.ratelimit().get().await
      .expect("Failed to get rate limit");

    for capture in captures {
      let owner = capture.name("owner")
        .and_then(|m| Some(m.as_str()))
        .unwrap_or("OpenTabletDriver");

      let repo = capture.name("repo")
        .and_then(|m| Some(m.as_str()))
        .unwrap_or("OpenTabletDriver");

      let issues = client.issues(owner, repo);
      let prs = client.pulls(owner, repo);

      let issue_num = capture.name("num")
        .and_then(|m| m.as_str().parse::<u64>().ok());

      if let Some(issue_num) = issue_num {
        if ratelimit.rate.remaining > 2 {
          if let Ok(pr) = prs.get(issue_num).await {
            embeds.push(pr.embed())
          } else if let Ok(issue) = issues.get(issue_num).await {
            embeds.push(issue.embed())
          }
        }
      }
    }

    if !embeds.is_empty() {
      return Some(embeds)
    }
  }

  None
}

trait Embeddable {
  fn embed(&self) -> CreateEmbed;
  fn get_title(&self) -> String;
  fn get_description(&self) -> String;
  fn get_colour(&self) -> Colour;
}

impl Embeddable for Issue {
  fn embed(&self) -> CreateEmbed {
    let mut default = CreateEmbed::default();
    let embed = default
      .title(format!("#{}: {}", self.number, self.title))
      .description(self.get_description())
      .url(self.html_url.as_str())
      .colour(self.get_colour())
      .author(|a| a
        .name(&self.user.login)
        .url(&self.user.url)
        .icon_url(&self.user.avatar_url)
      );

    if let Some(milestone) = &self.milestone {
      embed.field("Milestone", &milestone.title, true);
    }

    if !self.labels.is_empty() {
      let mut label_list = vec![];
      for label in &self.labels {
        label_list.push(label.name.clone());
      }

      let value = format!("`{}`", label_list.join("`, `"));
      embed.field("Labels", value, true);
    }

    embed.to_owned()
  }

  fn get_title(&self) -> String {
    format!("#{}: {}", self.number, self.title)
  }

  fn get_description(&self) -> String {
    let body = self.body.as_deref().unwrap_or_default();

    let mut description = String::default();
    for line in body.split("\n").take(15) {
      description.push_str(&format!("{}\n", line));
    }

    description.shrink_to(4096);
    description
  }

  fn get_colour(&self) -> Colour {
    match self.closed_at {
      Some(_) => CLOSED_COLOUR,
      None => OPEN_COLOUR
    }
  }
}

impl Embeddable for PullRequest {
  fn embed(&self) -> CreateEmbed {
    let mut description = self.body.clone().unwrap_or_default();
    description.shrink_to(4096);

    let mut default = CreateEmbed::default();
    let embed = default
      .title(self.get_title())
      .description(self.get_description())
      .colour(self.get_colour());

    if let Some(user) = &self.user {
      embed.author(|a| a
        .name(&user.login)
        .url(&user.url)
        .icon_url(&user.avatar_url)
      );
    }

    if let Some(url) = &self.html_url {
      embed.url(url.as_str());
    }

    if let Some(milestone) = &self.milestone {
      embed.field("Milestone", &milestone.title, true);
    }

    if let Some(labels) = &self.labels {
      let mut label_list = vec![];

      for label in labels {
        label_list.push(label.name.clone());
      }

      let value = format!("`{}`", label_list.join("`, `"));
      embed.field("Labels", value, true);
    }

    embed.to_owned()
  }

  fn get_title(&self) -> String {
    match &self.title {
      Some(title) => format!("#{}: {}", self.number, title),
      None => format!("#{}", self.number)
    }
  }

  fn get_description(&self) -> String {
    let body = self.body.as_deref().unwrap_or_default();

    let mut description = String::default();
    for line in body.split("\n").take(15) {
      description.push_str(&format!("{}\n", line));
    }

    description.shrink_to(4096);
    description
  }

  fn get_colour(&self) -> Colour {
    match self.closed_at {
      Some(_) => match self.merged_at {
        Some(_) => RESOLVED_COLOUR,
        None => CLOSED_COLOUR
      },
      None => OPEN_COLOUR
    }
  }
}
