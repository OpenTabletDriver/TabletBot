use serde::{Deserialize, Serialize};
use serde_json::{from_reader, to_writer_pretty};
use serenity::builder::CreateEmbed;
use serenity::prelude::TypeMapKey;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::path::Path;

pub trait Embeddable {
    fn embed(&self) -> CreateEmbed;
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Snippet {
    pub id: String,
    pub title: String,
    pub content: String,
}

impl Snippet {
    pub fn format_output(&self) -> String {
        format!("{}: {}", self.id, self.title)
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct BotState {
    pub snippets: Vec<Snippet>,
    pub issue_prefixes: HashMap<String, RepositoryDetails>,
}

impl TypeMapKey for BotState {
    type Value = BotState;
}

#[derive(Deserialize, Clone, Serialize, Default)]
pub struct RepositoryDetails {
    pub owner: String,
    pub name: String,
}

impl RepositoryDetails {
    pub fn get(&self) -> (&String, &String) {
        (&self.owner, &self.name)
    }
}

impl BotState {
    pub fn get_path() -> String {
        let pwd = env::current_dir().unwrap().to_string_lossy().to_string();
        let data_root = env::var("TABLETBOT_DATA").unwrap_or(pwd);

        match env::var("TABLETBOT_STATE") {
            Ok(path) => path,
            Err(_) => format!("{data_root}/state.json"),
        }
    }

    pub fn read() -> BotState {
        let path_str = Self::get_path();
        let path = Path::new(&path_str);

        if path.exists() {
            let file = File::open(path).unwrap();
            from_reader(file).unwrap()
        } else {
            BotState::default()
        }
    }

    pub fn write(&self) {
        let path_str = Self::get_path();
        let path = Path::new(&path_str);

        if path.exists() {
            fs::remove_file(path).expect("Failed to delete old file.");
        }

        let writer = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path);

        match writer {
            Ok(writer) => match to_writer_pretty(writer, self) {
                Ok(()) => println!("Successfully saved state to '{path_str}'",),
                Err(e) => println!("Failed to save state to '{path_str}': {e}"),
            },
            Err(e) => println!("Unable to write state to '{path_str}': {e}"),
        };
    }
}
