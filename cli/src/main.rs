extern crate futures;
extern crate tokio;
extern crate getopts;
extern crate cpal;
extern crate rb;
extern crate nix;
extern crate terminal_size;

extern crate hex_conf;
extern crate hex_database;
extern crate hex_music_container;

mod audio;
mod play;
mod modify;
mod sync;

use std::io::{self, Write};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use hex_database::{Instance, View, search::SearchQuery, Track, GossipConf};

use getopts::Options;

fn main() {
    let (conf, path) = match hex_conf::Conf::new() {
        Ok(x) => x,
        Err(err) => {
            eprintln!("Error: Could not load configuration {:?}", err);
            (hex_conf::Conf::default(), PathBuf::from("/opt/music/"))
        }
    };
    let data_path = path.join("data");
    let db_path = path.join("music.db");

    let args: Vec<String> = env::args().collect();
    let mut gossip = GossipConf::new();
    
    if let Some(ref peer) = conf.peer {
        gossip = gossip.addr((conf.host, peer.port));
        gossip = gossip.id(peer.id());
        gossip = gossip.network_key(peer.network_key());
    }

    let instance = Instance::from_file(&db_path, gossip);
    let view = instance.view();

    // print overview of the database
    if args.len() == 1 {
        print_overview(&view);
        return;
    }

    // in case the arguments are beginning with a value, we assume a search 
    let mut search_pattern = args.iter().skip(1)
        .take_while(|x| !x.contains("-")).cloned()
        .collect::<Vec<String>>().join(" ");

    // now build the option pattern
    let mut opts = Options::new();
    opts.optopt("s", "search", "search for tracks", "QUERY");
    opts.optopt("a", "action", "execute a certain action", "delete|modify|show|play|sync");
    opts.optflag("h", "help", "hex command line");
    let matches = opts.parse(&args[1..]).unwrap();

    if let Some(query) = matches.opt_str("s") {
        search_pattern = query;
    }

    let mut action = "show".into();
    if let Some(new_action) = matches.opt_str("a") {
        action = new_action;
    }

    let query = SearchQuery::new(&search_pattern);
    let mut query = view.search_prep(query).unwrap();
    let tracks: Vec<Track> = view.search(&mut query).collect();

    match action.as_ref() {
        "show" => {
            show_tracks(&search_pattern, tracks);
        },
        "delete" => {
            delete_tracks(&view, &data_path, tracks);
        },
        "sync" => {
            sync::sync_tracks(tracks, instance, data_path);
        },
        "play" => {
            play::play_tracks(data_path.clone(), tracks, instance);
        },
        "modify" => {
            modify::modify_tracks(&view, tracks);
        },
        _ => {
            println!("Unsupported action!");
            return;
        }
    }

}

fn show_tracks(query: &str, tracks: Vec<Track>) {
    println!("Found {} tracks for query: `{}`", tracks.len(), query);
    println!("");

    for track in tracks {
        if let (Some(ref title), Some(ref interpret)) = (&track.title, &track.interpret) {
            println!("\t{} ## {}", title, interpret);
        }
    }

}

fn delete_tracks(db: &View, data_path: &Path, tracks: Vec<Track>) {
    print!("Do you really want to delete {} tracks [n]: ", tracks.len());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            if input != "y\n" {
                return;
            }
        },
        Err(err) => {
            eprintln!("Error: {}", err);
            return;
        }
    }

    for track in tracks {
        db.delete_track(track.key).unwrap();

       if fs::remove_file(data_path.join(track.key.to_path())).is_err() {
           eprintln!("Error: Could not remove file of track {}", track.key.to_string());
       }
    }
}

fn print_overview(db: &View) {
    let mut tracks = db.get_tracks();
    tracks.sort_by(|a, b| a.favs_count.cmp(&b.favs_count).reverse());

    let duration = tracks.iter().fold(0.0, |y,x| y + x.duration);

    println!(" => Found {} tracks in total length {} min", tracks.len(), (duration / 60.0).floor());

    for track in tracks.iter().take(10) {
        if let (Some(ref title), Some(ref interpret)) = (&track.title, &track.interpret) {
            println!("\t{} ## {}", title, interpret);
        }
    }

    println!("");

    let playlists = db.get_playlists();

    println!(" => Found {} playlists:", playlists.len());

    for pl in playlists {
        println!("\t{} with {} tracks", pl.title, pl.tracks.len());
    }
}