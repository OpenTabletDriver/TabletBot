use octocrab::models::issues::Issue;
use octocrab::models::pulls::PullRequest;
use regex::Regex;
use serenity::builder::CreateEmbed;
use serenity::model::prelude::Message;
use serenity::prelude::Context;
use serenity::utils::Colour;

const REPO_OWNER: &str = "OpenTabletDriver";
const REPO_NAME: &str = "OpenTabletDriver";

const OPEN_COLOUR: Colour = Colour(0x238636);
const RESOLVED_COLOUR: Colour = Colour(0x8957e5);
const CLOSED_COLOUR: Colour = Colour(0xda3633);

pub async fn message(ctx: &Context, message: &Message) {
  if let Some(embeds) = issue_embeds(message).await {
    let typing = message.channel_id.start_typing(&ctx.http)
      .expect("Failed to start typing");

    message.channel_id.send_message(&ctx.http, |f| f
      .reference_message(message)
      .set_embeds(embeds)
    ).await.expect("Failed to reply with github embed");

    typing.stop().expect("Failed to stop typing");
  }
}

async fn issue_embeds(message: &Message) -> Option<Vec<CreateEmbed>> {
  let mut embeds: Vec<CreateEmbed> = vec![];
  let client = octocrab::instance();
  let ratelimit = client.ratelimit();
  let issues = client.issues(REPO_OWNER, REPO_NAME);
  let prs = client.pulls(REPO_OWNER, REPO_NAME);

  let regex = Regex::new(r#" ?#([0-9]+[0-9]) ?"#)
    .expect("Expected numbers regex");

  for capture in regex.captures_iter(&message.content) {
    if let Some(m) = capture.get(1) {
      let issue_num = m.as_str().parse::<u64>()
        .expect("Match is not a number");

      let ratelimit = ratelimit.get().await
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

trait Embeddable {
  fn embed(&self) -> CreateEmbed;
  fn get_title(&self) -> String;
  fn get_description(&self) -> String;
  fn get_colour(&self) -> Colour;
  fn get_labels(&self) -> Option<String>;
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

    if let Some(labels) = self.get_labels() {
      embed.field("Labels", labels, true);
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

  fn get_labels(&self) -> Option<String> {
    if !self.labels.is_empty() {
      let labels = &self.labels.iter()
      .map(|l| l.name.clone())
      .collect::<Vec<String>>();

      Some(format!("`{}`", labels.join("`, `")))
    } else {
      None
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

    if let Some(labels) = self.get_labels() {
      embed.field("Labels", labels, true);
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

  fn get_labels(&self) -> Option<String> {
    if let Some(labels) = &self.labels {
      if !labels.is_empty() {
        let labels = labels.iter()
        .map(|l| l.name.clone())
        .collect::<Vec<String>>();

        return Some(format!("`{}`", labels.join("`, `")))
      }
    }

    None
  }
}
