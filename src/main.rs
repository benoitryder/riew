#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(debug_assertions, windows_subsystem = "console")]

use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Rust image viewer")]
struct Cli {
    /// browse directory of provided file
    #[arg(short, long, value_name = "FILE", group = "input")]
    directory: Option<PathBuf>,
    /// browse given files
    #[arg(value_name = "FILE", group = "input")]
    files: Option<Vec<PathBuf>>,
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();

    let paths: Vec<_> =
        if let Some(file) = cli.directory {
            if let Some(parent) = file.parent() {
                let parent = parent.to_owned();
                vec![file, parent]
            } else {
                vec![file]
            }
        } else if let Some(files) = cli.files {
            files.into_iter().collect()
        } else {
            vec![PathBuf::from("")]
        };
    riew::App::init(paths)?.run()?;

    Ok(())
}

