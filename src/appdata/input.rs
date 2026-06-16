use std::{
    io::{IsTerminal, Read},
    os::unix::fs::PermissionsExt,
};

use serde::{Deserialize, Serialize};

use crate::cli::Cli;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct InputItems(pub Vec<InputItem>);

impl InputItems {
    pub fn new(cli: &Cli) -> Self {
        if cli.input {
            Self(vec![])
        } else if cli.path_launcher {
            Self::from_path()
        } else if cli.json_in {
            Self::from_json_in()
        } else {
            Self::from_input()
        }
    }

    fn get_stdin() -> String {
        let mut input = String::new();
        match std::io::stdin().read_to_string(&mut input) {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("failed to get stdin: {e}");
                std::process::exit(1)
            }
        }

        input
    }

    fn from_input() -> InputItems {
        if std::io::stdin().is_terminal() {
            tracing::warn!("expected input from a pipe, you might wanna run --path-launcher");
            std::process::exit(1);
        }

        InputItems(
            Self::get_stdin()
                .lines()
                .map(|v| InputItem::new(v.to_string(), serde_json::Value::String(v.to_string())))
                .collect(),
        )
    }

    fn from_json_in() -> InputItems {
        if std::io::stdin().is_terminal() {
            tracing::warn!(
                r#"expected input from a pipe in form of [{{"display": "Name", "raw": 69}}]"#
            );

            std::process::exit(1);
        }

        let input = Self::get_stdin();

        match serde_json::from_str(&input) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("input is invalid JSON: {e}");
                std::process::exit(1);
            }
        }
    }

    fn from_path() -> InputItems {
        let mut bins = Vec::new();

        let path = match std::env::var("PATH") {
            Ok(v) => v,
            Err(_) => {
                tracing::error!("No PATH set");
                std::process::exit(1);
            }
        };
        for dir in path.split(':') {
            let entries = match std::fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for entry in entries {
                let entry = match entry {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("{e}");
                        continue;
                    }
                };
                let path = entry.path();

                if path.is_file() {
                    let metadata = match std::fs::metadata(&path) {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!("{e}");
                            continue;
                        }
                    };
                    let permissions = metadata.permissions();
                    if permissions.mode() & 0o111 != 0
                        && let Some(name) = entry.file_name().to_str()
                    {
                        bins.push(InputItem::new(
                            name.to_owned(),
                            serde_json::Value::String(path.display().to_string()),
                        ));
                    }
                }
            }
        }

        InputItems(bins)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputItem {
    display: String,
    raw: serde_json::Value,
}

impl InputItem {
    pub fn new(display: String, raw: serde_json::Value) -> Self {
        Self { display, raw }
    }

    pub fn display(&self) -> &str {
        &self.display
    }

    pub fn raw(&self) -> &serde_json::Value {
        &self.raw
    }
}

#[derive(Default, Debug)]
pub struct Input {
    pub dirty: bool,

    input: String,

    inputs: InputItems,
    filtered_inputs: Vec<InputItem>,

    selected_index: u32,
}
impl Input {
    pub fn new(inputs: InputItems) -> anyhow::Result<Self> {
        let mut new = Self {
            dirty: true,

            input: String::new(),

            inputs,
            filtered_inputs: vec![],

            selected_index: 0,
        };
        new.update_bins();
        Ok(new)
    }

    pub fn pop(&mut self) {
        if self.input().is_empty() {
            return;
        }

        self.input.pop();
        self.selected_index = 0;
        self.update_bins();

        self.dirty = true;
    }

    pub fn push(&mut self, str: &str) {
        if str.is_empty() {
            return;
        }

        self.input.push_str(str);
        self.selected_index = 0;
        self.update_bins();

        self.dirty = true;
    }

    pub fn move_left(&mut self) {
        let old = self.selected_index();

        self.selected_index = self.selected_index().saturating_sub(1);

        if old != self.selected_index() {
            self.dirty = true;
        }
    }

    pub fn move_right(&mut self) {
        let old = self.selected_index();

        let max_index = self.filtered_inputs().len().saturating_sub(1) as u32;
        self.selected_index = (self.selected_index() + 1).min(max_index);

        if old != self.selected_index() {
            self.dirty = true;
        }
    }

    pub fn update_bins(&mut self) {
        let input = self.input.to_lowercase();

        let mut bins: Vec<(InputItem, String)> = self
            .inputs
            .0
            .iter()
            .filter(|s| {
                if input.is_empty() {
                    true
                } else {
                    self.input.is_empty() || s.display.contains(&self.input)
                }
            })
            .map(|s| (s.clone(), s.display.to_lowercase()))
            .collect();

        bins.sort_by(|a, b| {
            let score = |s: &str| {
                if !input.is_empty() && s.starts_with(&input) {
                    0
                } else if !input.is_empty() && s.contains(&input) {
                    1
                } else {
                    2
                }
            };

            score(&a.1)
                .cmp(&score(&b.1))
                .then_with(|| a.0.display.cmp(&b.0.display))
        });

        let bins = bins.into_iter().map(|(orig, _)| orig).collect();

        self.filtered_inputs = bins;
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn filtered_inputs(&self) -> &[InputItem] {
        &self.filtered_inputs
    }

    pub fn selected_index(&self) -> u32 {
        self.selected_index
    }
}
