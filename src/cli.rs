use std::env;
use std::io::{Write, Read};
use std::fs;
use std::path::PathBuf;
use hex_lib::Workspace;
use clap::{Arg, App, SubCommand, AppSettings};

fn main() {
    let matches =
    App::new("Hex command line interface")
        .version("0.1")
        .author("Lorenz Schmidt, <lorenz.schmidt@mailbox.org>")
        .about("Controls the local music library")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(SubCommand::with_name("init")
            .about("Initialize a new music workspace")
            .arg(Arg::with_name("PATH")
                .help("Sets the input path")
                .required(true)
                .index(1))
        )
        .subcommand(SubCommand::with_name("playlist")
            .about("Manage playlists")
            .arg(Arg::with_name("PATH")
                .help("Sets the input path")
                .index(1))
        )
        .get_matches();

    match matches.subcommand() {
        ("init", Some(sub_match)) => {
            let full_path = sub_match.value_of("PATH").unwrap();
            // parse path
            let mut full_path = PathBuf::from(full_path.to_string());

            if !full_path.is_absolute() {
                full_path = PathBuf::from(env::current_dir().unwrap()).join(full_path);
            }

            if full_path.join("Hex.toml").exists() {
                eprintln!(" => Workspace in {} already exists!", full_path.to_str().unwrap());
                return;
            }

            println!(" => Initialize workspace in {}", full_path.to_str().unwrap());

            // create root and files/ folder
            fs::create_dir_all(&full_path).unwrap();
            fs::create_dir_all(&full_path.join("files")).unwrap();

            // create Hex.toml file
            let mut f = fs::File::create(&full_path.join("Hex.toml")).unwrap();
            f.write("editor = \"vim\"\n\n".as_bytes()).unwrap();
        },

        ("playlist", Some(sub_match)) => {
            let full_path = env::current_dir().unwrap();

            if !full_path.join("Hex.toml").exists() {
                eprintln!("No music workspace: Hex.toml missing");

                return;
            }

            let workspace = Workspace::from_path(&full_path);

            match sub_match.value_of("PATH") {
                Some(paths) => {},
                None => {
                    for p in workspace.playlists() {
                    }
                }
            }
        },
        _ => {
        }
    }
}
