use std::path::PathBuf;

use tokio;
use futures::future::Future;

use hex_database::{Track, View};

pub fn sync_tracks(path: PathBuf, tracks: Vec<Track>, view: &View) {
    let length = tracks.len();

    for track in tracks {
        if path.join(track.key.to_path()).exists() {
            continue;
        }

        tokio::run(
            view.ask_for_file(track.key.clone())
                .map(|title| println!("Synchronized {:?}", title))
                .map_err(|err| println!("Could not synchronize {:?}", err))
        );
    }


    println!("Finished - {} tracks synchronised!", length);
}
