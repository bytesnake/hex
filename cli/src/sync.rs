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

        let (t1, t2) = (track.title.clone(), track.title.clone());
        tokio::run(
            view.ask_for_file(track.key.clone())
                .map(move |_| println!("Synchronized {:?}", t1))
                .map_err(move |err| println!("Could not synchronize {:?}, because {:?}", t2, err))
        );
    }


    println!("Finished - {} tracks synchronised!", length);
}
