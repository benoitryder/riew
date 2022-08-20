use std::path::PathBuf;
use clap::{App, Arg, ArgGroup};


fn main() -> Result<(), String> {
    let appm = App::new("riew")
        .about("Rust image viewer")
        .arg(Arg::with_name("directory")
            .short('d')
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
            let path = PathBuf::from(file);
            if let Some(parent) = path.parent() {
                let parent = parent.to_owned();
                vec![path, parent]
            } else {
                vec![path]
            }
        } else if let Some(files) = appm.values_of("files") {
            files.into_iter().map(PathBuf::from).collect()
        } else {
            vec![PathBuf::from("")]
        };
    riew::App::init(paths)?.run()?;

    Ok(())
}

