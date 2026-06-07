use std::{env, fs};

use anyhow::Context;

pub fn get_bin_names() -> anyhow::Result<Vec<String>> {
    let mut bins = Vec::new();

    for dir in env::var("PATH").context("PATH not set")?.split(':') {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                bins.push(name.to_owned());
            }
        }
    }

    Ok(bins)
}
