use octocrab::models::{issues::Issue, pulls::PullRequest};
use poise::serenity_prelude::{Colour, CreateEmbed, CreateEmbedAuthor};

use crate::structures::Embeddable;
use std::fmt::Write;

const OPEN_COLOUR: Colour = Colour::new(0x238636);
const RESOLVED_COLOUR: Colour = Colour::new(0x8957e5);
const CLOSED_COLOUR: Colour = Colour::new(0xda3633);

pub(super) trait Document {
    fn get_title(&self) -> String;
    fn get_content(&self) -> String;
    fn get_colour(&self) -> Colour;
    fn get_labels(&self) -> Option<String>;
}

impl Embeddable for Issue {
    fn embed(&self) -> CreateEmbed<'_> {
        let default = CreateEmbed::default();
        let author = CreateEmbedAuthor::new(&self.user.login)
            .url(self.user.url.as_str())
            .icon_url(self.user.avatar_url.as_str());
        let mut embed = default
            .title(self.get_title())
            .description(self.get_content())
            .url(self.html_url.as_str())
            .colour(self.get_colour())
            .author(author);

        if let Some(milestone) = &self.milestone {
            embed = embed.field("Milestone", &milestone.title, true);
        }

        if let Some(labels) = self.get_labels() {
            embed = embed.field("Labels", labels, true);
        }

        embed
    }
}

impl Document for Issue {
    fn get_title(&self) -> String {
        format!("#{}: {}", self.number, self.title)
    }

    fn get_content(&self) -> String {
        let body = self.body.as_deref().unwrap_or_default();

        let mut description = String::new();
        for line in body.split('\n').take(15) {
            writeln!(description, "{line}").unwrap();
        }

        description.shrink_to(4096);
        description
    }

    fn get_colour(&self) -> Colour {
        match self.closed_at {
            Some(_) => CLOSED_COLOUR,
            None => OPEN_COLOUR,
        }
    }

    fn get_labels(&self) -> Option<String> {
        if self.labels.is_empty() {
            None
        } else {
            let labels = &self
                .labels
                .iter()
                .map(|l| l.name.clone())
                .collect::<Vec<String>>();

            Some(format!("`{}`", labels.join("`, `")))
        }
    }
}

impl Embeddable for PullRequest {
    fn embed(&self) -> CreateEmbed<'_> {
        let mut description = self.body.clone().unwrap_or_default();
        description.shrink_to(4096);

        let default = CreateEmbed::default();
        let mut embed = default
            .title(self.get_title())
            .description(self.get_content())
            .colour(self.get_colour());

        if let Some(user) = &self.user {
            let author = CreateEmbedAuthor::new(user.login.clone())
                .url(user.url.as_str())
                .icon_url(user.avatar_url.as_str());
            embed = embed.author(author);
        }

        if let Some(url) = &self.html_url {
            embed = embed.url(url.as_str());
        }

        if let Some(milestone) = &self.milestone {
            embed = embed.field("Milestone", &milestone.title, true);
        }

        if let Some(labels) = self.get_labels() {
            embed = embed.field("Labels", labels, true);
        }

        embed
    }
}

impl Document for PullRequest {
    fn get_title(&self) -> String {
        match &self.title {
            Some(title) => format!("#{}: {}", self.number, title),
            None => format!("#{}", self.number),
        }
    }

    fn get_content(&self) -> String {
        let body = self.body.as_deref().unwrap_or_default();

        let mut content = String::new();
        for line in body.split('\n').take(15) {
            writeln!(content, "{line}").unwrap();
        }

        content.shrink_to(4096);
        content
    }

    fn get_colour(&self) -> Colour {
        match self.closed_at {
            Some(_) => match self.merged_at {
                Some(_) => RESOLVED_COLOUR,
                None => CLOSED_COLOUR,
            },
            None => OPEN_COLOUR,
        }
    }

    fn get_labels(&self) -> Option<String> {
        if let Some(labels) = &self.labels {
            if !labels.is_empty() {
                let labels = labels
                    .iter()
                    .map(|l| l.name.clone())
                    .collect::<Vec<String>>();

                return Some(format!("`{}`", labels.join("`, `")));
            }
        }

        None
    }
}
