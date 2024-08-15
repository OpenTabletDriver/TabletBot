use futures::future::join_all;
use regex::{Match, Regex};
use std::str::FromStr;
use std::{path::Path, sync::OnceLock};

use poise::serenity_prelude::{self as serenity, Colour, CreateEmbed, Http, Message};
use std::sync::Arc;

use crate::formatting::trim_indent;

use crate::FrameworkContext;

// A shade of purple.
const ACCENT_COLOUR: Colour = Colour::new(0x8957e5);

pub async fn message(framework: FrameworkContext<'_>, message: &Message) {
    let http = &framework.serenity_context.http.clone();

    let Some(file_refs) = get_file_refs(http.clone(), message) else {
        return;
    };

    // This is just cursed. I have no other way to explain this but its the only way I can figure
    // out how to satisfy the lifetimes.
    let embeds = join_all(file_refs.iter().map(FileReference::create_embed))
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let typing = message.channel_id.start_typing(http.clone());

    let content = serenity::CreateMessage::default()
        .embeds(embeds)
        .reference_message(message);
    let _ = message.channel_id.send_message(http, content).await;

    typing.stop();
}

fn get_file_refs(http: Arc<Http>, message: &Message) -> Option<Vec<FileReference<'_>>> {
    let typing = message.channel_id.start_typing(http);
    let mut embeds = vec![];

    if let Some(refs) = FileReference::try_from_str(&message.content) {
        for file_ref in refs {
            embeds.push(file_ref);
        }
    }

    typing.stop();

    if embeds.is_empty() {
        None
    } else {
        Some(embeds)
    }
}

async fn http_get_body_text(url: &String) -> Option<String> {
    match reqwest::get(url).await {
        Ok(res) => match res.text().await {
            Ok(content) => Some(content),
            Err(e) => {
                println!("Failed to get text: {e}");
                None
            }
        },
        Err(e) => {
            println!("Failed to get response: {e}");
            None
        }
    }
}

fn try_parse<F: FromStr + Clone>(m: Option<Match>) -> Option<F> {
    if let Some(m) = m {
        if let Ok(f) = m.as_str().parse::<F>() {
            return Some(f.to_owned());
        }
    }

    None
}

struct FileReference<'a> {
    owner: &'a str,
    repo: &'a str,
    git_ref: &'a str,
    path: &'a str,
    start: usize,
    end: Option<usize>,
}

impl FileReference<'_> {
    pub fn try_from_str(text: &str) -> Option<Vec<FileReference>> {
        let r = get_file_reference_regex();

        let files: Vec<FileReference> = r
            .captures_iter(text)
            .map(|capture| FileReference {
                owner: capture.get(1).expect("Expected owner").as_str(),
                repo: capture.get(2).expect("Expected repo").as_str(),
                git_ref: capture.get(3).expect("Expected git ref").as_str(),
                path: capture.get(4).expect("Expected file path").as_str(),
                start: try_parse::<usize>(capture.get(5)).expect("Expected start line"),
                end: try_parse::<usize>(capture.get(6)),
            })
            .collect();

        if files.is_empty() {
            None
        } else {
            Some(files)
        }
    }

    pub async fn create_embed(&self) -> Option<CreateEmbed> {
        let extension = self.get_extension();

        if let Some(mut content) = self.display().await {
            content.shrink_to(4096 - 8 - extension.len());

            let description = format!("```{extension}\n{content}\n```");

            let mut default = CreateEmbed::default();
            default = default
                .title(self.path)
                .description(description)
                .colour(ACCENT_COLOUR);

            Some(default)
        } else {
            None
        }
    }

    fn get_extension(&self) -> String {
        Path::new(self.path)
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }

    pub async fn display(&self) -> Option<String> {
        let url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            self.owner, self.repo, self.git_ref, self.path
        );
        println!("Downloading content: {url}");

        if let Some(content) = http_get_body_text(&url).await {
            let lines: Vec<&str> = content.split('\n').collect();
            let start = self.start - 1;

            if let Some(end) = self.end {
                if end <= start {
                    None
                } else {
                    Some(trim_indent(&lines[start..end]))
                }
            } else {
                Some(lines[start].trim_start().to_string())
            }
        } else {
            None
        }
    }
}

fn get_file_reference_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"https://github.com/(.+?)/(.+?)/blob/(.+?)/(.+?)#L([0-9]+)(?:-L([0-9]+))?")
            .unwrap()
    })
}
