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
               .args(&["directory", "files"])
               .required(true))
        .get_matches();

    let paths: Vec<_> =
        if appm.is_present("directory") {
            let file = appm.value_of("directory").unwrap();
            vec![Path::new(file), Path::new(".")]
        } else {
            appm.values_of("files").unwrap().into_iter().map(|f| Path::new(f)).collect()
        };

    riew::App::init(&paths)?.run()?;

    Ok(())
}

