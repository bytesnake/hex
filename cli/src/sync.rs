use std::path::Path;

use tokio;
use futures::future::Future;

use hex_database::{Track, Files};

pub fn sync_tracks(files: &Files, path: &Path, tracks: Vec<Track>) {
    let length = tracks.len();

    for track in tracks {
        if path.join(track.key.to_path()).exists() {
            continue;
        }

        let (t1, t2) = (track.title.clone(), track.title.clone());
        tokio::run(
            files.ask_for_file(track.key.clone())
                .map(move |_| println!("Synchronized {:?}", t1))
                .map_err(move |err| println!("Could not synchronize {:?}, because {:?}", t2, err))
        );
    }


    println!("Finished - {} tracks synchronised!", length);
}
