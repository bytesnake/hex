#[macro_use] extern crate log;

mod audio;
mod play;
mod modify;
mod sync;
mod store;

use std::thread;
use std::io::{self, Write, BufRead};
use std::fs;
use std::path::{Path, PathBuf};
use futures::stream::Stream;
use futures::future::IntoFuture;
use futures::future::Future;

use hex_database::{Instance, Reader, Writer, search::SearchQuery, Track, GossipConf, Playlist};

fn main() {
    env_logger::init();

    let (conf, path) = match hex_conf::Conf::new() {
        Ok(x) => x,
        Err(err) => {
            eprintln!("Error: Could not load configuration {:?}", err);
            (hex_conf::Conf::default(), PathBuf::from("/opt/music/"))
        }
    };
    let data_path = path.join("data");
    let db_path = path.join("music.db");

    let mut gossip = GossipConf::new();
    
    if let Some(ref peer) = conf.peer {
        gossip = gossip.addr((conf.host, peer.port));
        gossip = gossip.id(peer.id());
        gossip = gossip.network_key(peer.network_key());
        gossip = gossip.discover(peer.discover);
        gossip = gossip.contacts(peer.contacts.clone());
    }

    let instance = Instance::from_file(&db_path, gossip);
    let (read, write, files) = (instance.reader(),instance.writer(),instance.files());
    let mut prev_lines = Vec::new();

    // spawn in background
    thread::spawn(|| tokio::run(instance.for_each(|_| Ok(())).into_future().map_err(|_| ())));

    'outer: loop {
        print!(" > ");
        io::stdout().flush().ok().expect("Could not flush stdout");

        // get next line
        let line; 
        
        {
            let stdin = io::stdin();
            let mut iterator = stdin.lock().lines();

            loop {
                match iterator.next() {
                    Some(Ok(e)) => { line = e; break; },
                    Some(Err(_)) => continue,
                    None => {
                        println!("");
                        continue 'outer
                    }
                }
            }
        }

        prev_lines.push(line.clone());

        let mut args: Vec<&str> = line.splitn(2, ' ').collect();
        if args.len() == 0 {
            continue;
        } else if args.len() == 1 {
            args.push("");
        }

        let query = SearchQuery::new(&args[1]);
        let mut query = read.search_prep(query).unwrap();
        let tracks: Vec<Track> = read.search(&mut query).collect();

        let data_path = data_path.clone();
        match args[0] {
            "" => {
                print_overview(&read);
            },
            "search" => {
                show_tracks(&args[1], tracks);
            },
            "delete" => {
                delete_tracks(&write, &data_path, tracks);
            },
            "add-playlist" => {
                add_playlist(&read, &write, tracks);
            },
            "sync" => {
                sync::sync_tracks(&files, &data_path, tracks);
            },
            "play" => {
                play::play_tracks(&files, &data_path, tracks);
            },
            "modify" => {
                modify::modify_tracks(&write, tracks);
            },
            "modify-playlist" => {
                if let Ok((playlist,tracks)) = read.get_playlist_by_title(&args[1]) {
                    modify::modify_playlist(&write, playlist, tracks);
                } else {
                    println!("Playlist {} not found!", args[1]);
                }
            },
            "modify-tokens" => {
                modify::modify_tokens(&write, &read);
            },

            "store" => {
                store::store(&write, Path::new(args[1]), &data_path);
            },
            "quit" | "q" | "exit" | "bye" => {
                println!("Bye, have a nice day!");
                return;
            },
            _ => {
                println!("Unsupported action, use with <search|delete|add-playlist|sync|play|modify|store|quit>");
            }
        }
    }
}

fn show_tracks(query: &str, tracks: Vec<Track>) {
    println!("Found {} tracks for query: `{}`", tracks.len(), query);
    println!("");

    for track in tracks {
        if let (Some(ref title), Some(ref album), Some(ref interpret)) = (&track.title, &track.album, &track.interpret) {
            println!("\t{} ({}) ## {}", title, album, interpret);
        }
    }

}

fn add_playlist(read: &Reader, write: &Writer, tracks: Vec<Track>) {
    println!("Create new playlist with {} tracks", tracks.len());

    let last_key = read.last_playlist_key().unwrap();
    let pl = Playlist {
        key: last_key + 1,
        title: "New Playlist".into(),
        desc: None,
        tracks: tracks.into_iter().map(|x| x.key).collect(),
        origin: vec![0; 16]
    };

    write.add_playlist(pl).unwrap();
}

fn delete_tracks(write: &Writer, data_path: &Path, tracks: Vec<Track>) {
    print!("Do you really want to delete {} tracks [n]: ", tracks.len());
    io::stdout().flush().unwrap();

    let stdin = io::stdin();
    let lock = stdin.lock();
    let mut lines = lock.lines();

    match lines.next() {
        Some(Ok(input)) => {
            println!("Got {:?}", input);

            if input != "y" {
                return;
            }
        },
        _ => {
            return;
        }
    }

    for track in tracks {
        write.delete_track(track.key).unwrap();

       if fs::remove_file(data_path.join(track.key.to_path())).is_err() {
           eprintln!("Error: Could not remove file of track {}", track.key.to_string());
       }
    }
}

fn print_overview(read: &Reader) {
    let mut tracks = read.get_tracks();
    tracks.sort_by(|a, b| a.favs_count.cmp(&b.favs_count).reverse());

    let duration = tracks.iter().fold(0.0, |y,x| y + x.duration);

    println!(" => Found {} tracks in total length {} min", tracks.len(), (duration / 60.0).floor());

    for track in tracks.iter().take(10) {
        if let (Some(ref title), Some(ref interpret)) = (&track.title, &track.interpret) {
            println!("\t{} ## {}", title, interpret);
        }
    }

    println!("");

    let playlists = read.get_playlists();

    println!(" => Found {} playlists:", playlists.len());

    for pl in playlists {
        println!("\t{} with {} tracks", pl.title, pl.tracks.len());
    }
}
