use tokio;
use futures::future::Future;
use futures::stream;
use futures::stream::Stream;

use hex_database::{Track, View};

pub fn sync_tracks(tracks: Vec<Track>, view: &View) {
    let length = tracks.len();

    let futures = tracks.into_iter()
        .map(|track| view.ask_for_file(track.key.clone()).map(move |_| track.title.clone()));

    let stream = stream::futures_unordered(futures)
        .map(|title| println!("Synchronized {:?}", title))
        .map_err(|err| println!("Could not synchronize {:?}", err));

    tokio::run(stream.into_future().map(|_| ()).map_err(|_| ()));

    println!("Finished - {} tracks synchronised!", length);
}
