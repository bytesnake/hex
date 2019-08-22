use std::io::Write;
use std::path::PathBuf;
use std::slice;
use std::thread;
use std::fs::{self, File};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::sync::mpsc::{channel, Sender, Receiver};

use crate::error::*;

use hex_music_container::{Container, Configuration, error::Error as MusicError};
use hex_database::Track;

use threadpool::ThreadPool;

pub struct DownloadProgress {
    pub result: Result<(PathBuf, Track)>,
    pub num: usize
}


fn worker(track: Track, data_path: PathBuf) -> Result<(PathBuf, Track)> {
    let download_path = data_path.join("download");
    let file_path = data_path.join("data").join(&track.key.to_string());

    let file_path_out = download_path
        .join(&track.interpret.clone().unwrap_or("unknown".into()))
        .join(&track.album.clone().unwrap_or("unknown".into()));

    fs::create_dir_all(&file_path_out).unwrap();

    let file_path_out = file_path_out.join(track.title.clone().unwrap_or(track.key.to_string()));

    let file = File::open(&file_path)
        .map_err(|err| Error::Io(err))?;

    let mut container = Container::load(file)
        .map_err(|err| Error::MusicContainer(err))?;

    let mut out = File::create(&file_path_out)
        .map_err(|err| Error::Io(err))?;

    loop {
        match container.next_packet(Configuration::Stereo) {
            Ok(buf) => { 
                let buf: &[u8] = unsafe {
                    slice::from_raw_parts(
                        buf.as_ptr() as *const u8,
                        buf.len() * 2
                    )
                };

                out.write(&buf).unwrap();
            },
            Err(MusicError::ReachedEnd) => break,
            Err(err) => { return Err(Error::MusicContainer(err)); }
        }
    }

    let converted_file = file_path_out.with_extension("ogg");

    Command::new("ffmpeg")
        .arg("-y").arg("-loglevel").arg("0").arg("-nostats")
        .arg("-ar").arg("48k")
        .arg("-ac").arg("2")
        .arg("-f").arg("s16le")
        .arg("-i").arg(file_path_out.to_str().unwrap())
        .arg("-metadata").arg(&format!("title=\"{}\"", track.title.clone().unwrap_or(track.key.to_string())))
        .arg("-metadata").arg(&format!("album=\"{}\"", track.album.clone().unwrap_or("Unknown".into())))
        .arg("-metadata").arg(&format!("author=\"{}\"", track.interpret.clone().unwrap_or("Unknown".into())))
        .arg("-metadata").arg(&format!("composer=\"{}\"", track.composer.clone().unwrap_or("Unknown".into())))
        .arg(converted_file.to_str().unwrap())
        .spawn().expect("Could not start ffmpeg!").wait().unwrap();



    Ok((converted_file, track))
}

pub struct State {
    pub recv: Receiver<DownloadProgress>
}

impl State {
    pub fn new(tracks: Vec<Track>, data_path: PathBuf) -> State {
        let (sender, recv) = channel(200);

        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));

        for track in tracks {
            let mut sender = sender.clone();
            let data_path = data_path.clone();
            let counter = counter.clone();

            pool.execute(move || {
                let item = worker(track, data_path);
                let cnt = counter.fetch_add(1, Ordering::Relaxed);
                sender.try_send(DownloadProgress { result: item, num: cnt}).unwrap();
            });
        }

        State {
            recv
        }
    }
}
