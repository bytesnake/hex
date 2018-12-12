use std::io::{self, Write};
use std::path::PathBuf;
use std::fs::File;

use futures::Future;

use hex_database::{Track, Instance};

pub fn sync_tracks(tracks: Vec<Track>, mut instance: Instance, data_path: PathBuf) {
    let length = tracks.len();
    for track in tracks {
        print!("Syncing track: {}", track.key.to_string());
        io::stdout().flush().unwrap();

        if data_path.join(track.key.to_path()).exists() {
            println!(" already exists!");
            continue;
        }

        match instance.ask_for_file(track.key.to_vec()).wait() {
            Ok(buf) => {
                let mut file = File::create(data_path.join(track.key.to_path())).unwrap();
                file.write_all(&buf).unwrap();

                println!(" ok")
            },
            Err(err) => println!(" err {}", err)
        }
    }

    println!("Finished - {} tracks synchronised!", length);
}
