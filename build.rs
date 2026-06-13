use clap::{CommandFactory, ValueEnum};

const NAME: &str = "kymenu";

mod color {
    include!("src/color.rs");
}
include!("src/cli.rs");

fn main() -> Result<(), std::io::Error> {
    let out_dir = match std::env::var_os("OUT_DIR") {
        Some(outdir) => std::path::PathBuf::from(outdir),
        None => return Ok(()),
    };

    println!("{}", out_dir.display());

    let mut command = Cli::command();

    generate_shell_completions(&mut command, &out_dir)?;
    generate_man_page(command, &out_dir)?;

    Ok(())
}

fn generate_man_page(
    command: clap::Command,
    out_dir: &std::path::Path,
) -> Result<(), std::io::Error> {
    let man_folder = out_dir.join("man");
    std::fs::create_dir_all(&man_folder)?;

    let man = clap_mangen::Man::new(command);

    let mut buffer: Vec<u8> = Vec::new();
    man.render(&mut buffer)?;

    let path = man_folder.join(format!("{NAME}.1"));

    std::fs::write(&path, buffer)?;

    println!("Wrote man page to {}", path.display());

    Ok(())
}

fn generate_shell_completions(
    command: &mut clap::Command,
    out_dir: &std::path::Path,
) -> Result<(), std::io::Error> {
    let completions_folder = out_dir.join("completions");
    std::fs::create_dir_all(&completions_folder)?;

    for &shell in clap_complete::Shell::value_variants() {
        println!("Generating shell completions for {shell}");
        clap_complete::generate_to(shell, command, NAME, &completions_folder)?;
    }

    Ok(())
}
