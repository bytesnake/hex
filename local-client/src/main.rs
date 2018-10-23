extern crate hex_database;
extern crate hex_music_container;
extern crate clap;

use std::env;
use std::path::PathBuf;
use hex_database::Collection;

fn main() {
    let (path_data, path_db) = env::vars()
        .filter(|(key, _)| key == "HEX_PATH").map(|(_, a)| a).next()
        .map(|x| (PathBuf::from(&x).join("data"), PathBuf::from(&x).join("music.db")))
        .unwrap();

    let db = Collection::from_file(&path_db);

    let mut tracks = db.get_tracks();
    tracks.sort_by(|a, b| a.favs_count.cmp(&b.favs_count).reverse());

    let duration = tracks.iter().fold(0.0, |y,x| y + x.duration);

    println!(" => Found {} tracks with total length of {} min", tracks.len(), (duration / 60.0).floor());

    for track in tracks.iter().take(10) {
        if let (Some(ref title), Some(ref interpret)) = (&track.title, &track.interpret) {
            println!("\t{} ## {}", title, interpret);
        }
    }

    println!("");

    let playlists = db.get_playlists();

    println!(" => Found {} playlists:", playlists.len());

    for pl in playlists {
        println!("\t{} with {} tracks", pl.title, pl.count);
    }
}
