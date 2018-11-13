use std::io::{self, Write};
use std::path::Path;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use tokio;
use futures::Future;

use hex_sync::Peer;
use hex_database::Track;

pub fn sync_tracks(tracks: Vec<Track>, db_path: &Path, data_path: &Path, addr: SocketAddr, name: String) {
    let (mut peer, chain) = Peer::new(
        db_path.to_path_buf(), data_path.to_path_buf(), addr, name, false);

    thread::spawn(|| {
        tokio::run(chain);
    });

    thread::sleep(Duration::from_millis(3000));

    let length = tracks.len();
    for track in tracks {
        print!("Syncing track: {}", track.key.to_string());
        io::stdout().flush().unwrap();

        match peer.ask_for_track(track.key).wait() {
            Ok(_) => println!(" ok"),
            Err(err) => println!(" err {}", err)
        }
    }

    println!("Finished! {} tracks", length);
}
