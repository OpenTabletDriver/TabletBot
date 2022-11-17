use serde_json::{from_reader, to_writer_pretty};
use serde::{Deserialize, Serialize};
use serenity::prelude::TypeMapKey;
use std::{env, fs};
use std::fs::{File, OpenOptions};
use std::path::Path;
use crate::structures::{Snippet, ReactionRole};

#[derive(Deserialize, Serialize)]
pub struct State {
  pub snippets: Vec<Snippet>,
  pub reaction_roles: Vec<ReactionRole>
}

impl Default for State {
  fn default() -> State {
    Self {
      snippets: Vec::new(),
      reaction_roles: Vec::new()
    }
  }
}

impl TypeMapKey for State {
  type Value = State;
}

impl State {
  pub fn get_data_root() -> String {
    let pwd = env::current_dir().unwrap().to_string_lossy().to_string();
    env::var("TABLETBOT_DATA").unwrap_or(pwd)
  }

  fn get_path() -> String {
    match std::env::var("TABLETBOT_STATE") {
      Ok(path) => path,
      Err(_) => format!("{}/state.json", Self::get_data_root())
    }
  }

  pub fn read() -> Option<State> {
    Self::read_file(&Self::get_path())
  }

  pub fn write(&self) {
    Self::write_file(&self, &Self::get_path())
  }

  pub fn read_file(file_path: &str) -> Option<State> {
    let path = Path::new(file_path);

    if path.exists() {
      let file = File::open(file_path).unwrap();
      Some(from_reader(file).unwrap())
    } else {
      None
    }
  }

  pub fn write_file(&self, file_path: &str) {
    let path = Path::new(file_path);

    if path.exists() && let Err(e) = fs::remove_file(path) {
      panic!("Failed to overwrite file '{}': {}", file_path, e)
    }

    let result = OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open(path);

    match result {
      Ok(file) => to_writer_pretty(file, self).expect("Failed to write"),
      Err(e) => println!("Unable to write state to {}: {}", file_path, e)
    };
  }
}
