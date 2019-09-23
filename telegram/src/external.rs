use std::fs::File;
use std::io::{Read, Write};
use std::process::{Stdio, Command};
use std::path::PathBuf;
use std::thread;
use std::io::BufReader;
use std::io::BufRead;
use std::sync::Mutex;
use std::sync::Arc;

use hex_database::Writer;
use std::sync::mpsc::{channel, Sender, Receiver};

use rspotify::spotify::client::Spotify as SpotifyAPI;
use rspotify::spotify::oauth2::SpotifyClientCredentials;

use hex_database::{Track, utils::fingerprint_from_file};
use hex_music_container::{Configuration, Container};

type PseudoTrack = (String, String, String, String);
type PseudoPlaylist = (String, Vec<PseudoTrack>);
type CurrentPlaylist = (String, String);

fn get_playlist(spotify: &SpotifyAPI, key: &str) -> PseudoPlaylist {
    let playlists = spotify.playlist(&key, None, None);

    let mut tracks = Vec::new();
    let mut title = "".into();

    if let Ok(playlist) = playlists {
        for track in playlist.tracks.items {
            if let Some(id) = track.track.id {
                let album = track.track.album.name;
                let title = track.track.name;
                let artists = track.track.artists.into_iter().map(|x| x.name).collect::<Vec<String>>().join(", ");

                tracks.push((title, album, artists, id));
            }
        }

        title = playlist.name.clone();
    }

    (title, tracks)
}

pub struct ExternalMusic {
    sender: Sender<String>,
    current_playlist: Arc<Mutex<Option<CurrentPlaylist>>>
}

impl ExternalMusic {
    pub fn new(write: Writer, data_path: PathBuf, auth: hex_conf::SpotifyAPI) -> ExternalMusic {
        let client_credential = SpotifyClientCredentials::default()
            .client_id(&auth.id)
            .client_secret(&auth.secret)
            .build();

        let spotify_api = SpotifyAPI::default()
            .client_credentials_manager(client_credential)
            .build();

        let mut capture = Command::new("music_external")
            .stdin(Stdio::piped()).stdout(Stdio::piped())
            .spawn().expect("Could not find external music player!");

        let mut stdin = capture.stdin.take().unwrap();
        let mut stdout = BufReader::new(capture.stdout.take().unwrap());

        let mut init_str = String::new();
        stdout.read_line(&mut init_str).unwrap();

        let (sender, recv): (Sender<String>, Receiver<String>) = channel();
        let current_playlist = Arc::new(Mutex::new(None));
        let c2 = current_playlist.clone();

        thread::spawn(move || {

            while let Ok(playlist_key) = recv.recv() {
                let playlist = get_playlist(&spotify_api, &playlist_key);

                for metadata in playlist.1 {
                    *c2.lock().unwrap() = Some((playlist.0.clone(), metadata.0.clone()));
                    stdin.write(metadata.3.as_bytes()).unwrap();
                    stdin.write(b"\n").unwrap();
                    
                    let mut path_str = String::new();
                    stdout.read_line(&mut path_str).unwrap();
                    let path = PathBuf::from(&path_str[..path_str.len()-1]);
                    
                    println!("Add from path {:?}", path);

                    let mut file = File::open(&path).unwrap();
                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf).unwrap();
                    
                    let fingerprint = fingerprint_from_file(2, path).unwrap();
                    
                    let mut track = Track::empty(fingerprint, buf.len() as f64 / 48000.0 / 2.0);
                    track.title = Some(metadata.0);
                    track.album = Some(metadata.1);
                    track.interpret = Some(metadata.2.clone());
                    track.composer = Some(metadata.2.clone());
                    
                    let track_path = track.key.to_path();

                    //println!("Added track {:?}", track.title);

                    if let Err(err) = write.add_track(track) {
                        eprintln!("Could not add track: {:?}", err);
                    }
                    
                    let samples: &[i16] = unsafe { ::std::slice::from_raw_parts(buf.as_ptr() as *const i16, buf.len() / 2) };
                    
                    let file = File::create(data_path.join("data").join(track_path)).unwrap();

                    Container::save_pcm(Configuration::Stereo, samples.to_vec(), file, None).unwrap();
                }
                *c2.lock().unwrap() = None;
            }
        });

        ExternalMusic { sender, current_playlist }
    }

    pub fn add_playlist(&self, key: &str) {
        self.sender.send(key.into()).unwrap();
    }

    pub fn current_playlist(&self) -> Option<CurrentPlaylist> {
        self.current_playlist.lock().unwrap().clone()
    }
}
