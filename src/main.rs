extern crate sdl2;

use std::path::Path;
use clap::{App, Arg, ArgGroup};


fn main() -> Result<(), String> {
    let appm = App::new("riew")
        .about("Rust image viewer")
        .arg(Arg::with_name("directory")
            .short("d")
            .value_name("FILE")
            .help("browse directory of provided file"))
        .arg(Arg::with_name("files")
            .multiple(true)
            .value_name("FILE")
            .help("browse given files"))
        .group(ArgGroup::with_name("input")
            .args(&["directory", "files"]))
        .get_matches();

    let paths: Vec<_> =
        if let Some(file) = appm.value_of("directory") {
            let path = Path::new(file);
            if let Some(parent) = path.parent() {
                vec![path, parent]
            } else {
                vec![path]
            }
        } else if let Some(files) = appm.values_of("files") {
            files.into_iter().map(|f| Path::new(f)).collect()
        } else {
            vec![Path::new(".")]
        };

    riew::App::init(&paths)?.run()?;

    Ok(())
}

