use std::io::{Write, BufRead, BufReader};
use std::fs::{self, File};
use std::collections::HashMap;
use std::process::Command;
use hex_database::{Track, Reader, Writer, Playlist};

pub fn modify_tracks(write: &Writer, tracks: Vec<Track>) {
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

            write.update_track(
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

pub fn modify_playlist(write: &Writer, mut playlist: Playlist, tracks: Vec<Track>) {
    let mut map = HashMap::new();

    {
        let mut file = File::create("/tmp/cli_modify").unwrap();

        file.write(&format!("Playlist title: {}\n", playlist.title).as_bytes()).unwrap();
        file.write(&format!("Playlist description: {}\n", playlist.desc.unwrap_or("None".into())).as_bytes()).unwrap();

        file.write("Tracks:\n".as_bytes()).unwrap();
        for track in tracks.clone() {
            let line = format!(" => {} - {} ({})", track.key.to_string(), track.title.unwrap_or("".into()), track.album.unwrap_or("".into()));

            file.write(&format!("{}\n", line).as_bytes()).unwrap();
            map.insert(line, track.key.clone());
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
        let mut reader = BufReader::new(file).lines();

        if let Some(Ok(title)) = reader.next() {
            let mut splitted = title.split(":");
            if splitted.next() != Some("Playlist title") {
                println!("Playlist title not found!");
                return;
            }

            playlist.title = title.split(":").skip(1).next().unwrap_or("New Playlist").trim().to_string();
        } else {
            println!("Playlist file too short!");
            return;
        }

        if let Some(Ok(desc)) = reader.next() {
            let mut splitted = desc.split(":");
            if splitted.next() != Some("Playlist description") {
                println!("Playlist description not found!");
                return;
            }

            playlist.desc = splitted.next().map(|x| x.trim().to_string());
        } else {
            println!("Playlist file too short!");
            return;
        }

        if let Some(Ok(title)) = reader.next() {
            if title != "Tracks:" {
                println!("Track header not found!");
                return;
            }

            playlist.tracks.clear();
            while let Some(Ok(track)) = reader.next() {
                if let Some(key) = map.get(&track) {
                    playlist.tracks.push(*key);
                }
            }

            if let Err(err) = write.update_playlist(playlist.key, Some(playlist.title), playlist.desc, Some(playlist.tracks)) {
                eprintln!("Could not update playlist = {:?}", err);
            }
        }


    }

    fs::remove_file("/tmp/cli_modify").unwrap();
}

pub fn modify_tokens(write: &Writer, read: &Reader) {
    let tokens = read.get_tokens().unwrap();

    {
        let mut file = File::create("/tmp/cli_modify").unwrap();

        for (token, playlist) in tokens.clone() {
            let buf = format!("{} | {} | {}\n",
                token.token,
                token.last_use,
                playlist.map(|x| x.title).unwrap_or("None".into()));

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

        for (line, (mut token, playlist)) in BufReader::new(file).lines().zip(tokens.into_iter()) {
            let params: Vec<String> = line.unwrap().split("|").map(|x| x.trim().into()).collect();
            if params.len() != 3 {
                continue;
            }

            if params[2] == "None" {
                token.key = None;
                write.update_token2(token);
            } else if playlist.map(|x| x.title).unwrap_or("None".into()) != params[2] {
                if let Ok((new_playlist,_)) = read.get_playlist_by_title(&params[2]) {
                    token.key = Some(new_playlist.key);
                    write.update_token2(token);
                } else {
                    println!("Could not find playlist {}!", params[2]);
                }
            }
        }

        /*for (line, track) in BufReader::new(file).lines().zip(tracks.into_iter()) {
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

            write.update_track(
                track.key,
                if params[0] == "None" { None } else { Some(&params[0]) },
                if params[1] == "None" { None } else { Some(&params[1]) },
                if params[2] == "None" { None } else { Some(&params[2]) },
                if params[3] == "None" { None } else { Some(&params[3]) },
                if params[4] == "None" { None } else { Some(&params[4]) },
            ).unwrap();
        }*/
    }

    fs::remove_file("/tmp/cli_modify").unwrap();
}

