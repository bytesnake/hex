use std::io::{Write, BufRead, BufReader};
use std::fs::{self, File};
use std::process::Command;
use hex_database::{Track, Collection, TrackKey};

pub fn modify_tracks(db: &Collection, tracks: Vec<Track>) {
    {
        let mut file = File::create("/tmp/cli_modify").unwrap();

        for track in tracks {
            let buf = format!("{} | {} | {} | {} | {} | {}\n", 
                track.key.to_string(),
                track.title.unwrap_or("None".into()),
                track.album.unwrap_or("None".into()),
                track.interpret.unwrap_or("None".into()),
                track.people.unwrap_or("None".into()),
                track.composer.unwrap_or("None".into())
            );

            file.write(&buf.as_bytes()).unwrap();
        }
    }

    // open vim and edit all tracks
    Command::new("vim")
        .arg("-c").arg(":set wrap!")
        .arg("/tmp/cli_modify")
        .status().expect("Could not open file!");

    // apply changes to the database
    {
        let file = File::open("/tmp/cli_modify").unwrap();

        for line in BufReader::new(file).lines() {
            let params: Vec<String> = line.unwrap().split("|").map(|x| x.trim().into()).collect();
            if params.len() != 6 {
                continue;
            }

            db.update_track(
                TrackKey::from_str(&params[0]),
                if params[1] == "None" { None } else { Some(&params[1]) },
                if params[2] == "None" { None } else { Some(&params[2]) },
                if params[3] == "None" { None } else { Some(&params[3]) },
                if params[4] == "None" { None } else { Some(&params[4]) },
                if params[5] == "None" { None } else { Some(&params[5]) },
            ).unwrap();
        }
    }

    fs::remove_file("/tmp/cli_modify").unwrap();
}
