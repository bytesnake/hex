use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use hex_database::{Track, TrackKey};

pub fn sync_tracks(tracks: Vec<Track>, sender: Sender<TrackKey>, data_path: PathBuf) {
    let length = tracks.len();
    for track in tracks {
        print!("Syncing track: {}", track.key.to_string());
        io::stdout().flush().unwrap();

        sender.send(track.key.clone()).unwrap();

        while !data_path.join(track.key.to_path()).exists() {}

        println!(" ok");
    }

    println!("Finished - {} tracks synchronised!", length);
}
