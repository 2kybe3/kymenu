use std::{env, fs, os::unix::fs::PermissionsExt};

use anyhow::Context;

pub fn get_bin_names() -> anyhow::Result<Vec<String>> {
    let mut bins = Vec::new();

    let path = env::var("PATH").context("PATH not set")?;
    for dir in path.split(':') {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let metadata = fs::metadata(&path)?;
                let permissions = metadata.permissions();
                if permissions.mode() & 0o111 != 0
                    && let Some(name) = entry.file_name().to_str()
                {
                    bins.push(name.to_owned());
                }
            }
        }
    }

    Ok(bins)
}
