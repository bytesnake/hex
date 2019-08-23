use std::io::{Write, BufRead, BufReader};
use std::fs::{self, File};
use std::process::Command;
use hex_database::{Track, View, TrackKey};

pub fn modify_tracks(view: &View, tracks: Vec<Track>) {
    {
        let mut file = File::create("/tmp/cli_modify").unwrap();

        for track in tracks.clone() {
            let buf = format!("{} | {} | {} | {} | {}\n", 
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
        .arg("-c").arg(":%!column -t -o \"|\" -s \"|\"")
        .arg("/tmp/cli_modify")
        .status().expect("Could not open file!");

    // apply changes to the database
    {
        let file = File::open("/tmp/cli_modify").unwrap();

        for (line, track) in BufReader::new(file).lines().zip(tracks.into_iter()) {
            let params: Vec<String> = line.unwrap().split("|").map(|x| x.trim().into()).collect();
            if params.len() != 5 {
                continue;
            }

            // skip if there is no change
            if track.title.unwrap_or("None".into()) == params[0] && 
               track.album.unwrap_or("None".into()) == params[1] && 
               track.interpret.unwrap_or("None".into()) == params[2] && 
               track.people.unwrap_or("None".into()) == params[3] && 
               track.composer.unwrap_or("None".into()) == params[4] {
                continue;
            }

            view.update_track(
                track.key,
                if params[0] == "None" { None } else { Some(&params[0]) },
                if params[1] == "None" { None } else { Some(&params[1]) },
                if params[2] == "None" { None } else { Some(&params[2]) },
                if params[3] == "None" { None } else { Some(&params[3]) },
                if params[4] == "None" { None } else { Some(&params[4]) },
            ).unwrap();
        }
    }

    fs::remove_file("/tmp/cli_modify").unwrap();
}
