use std::io::Write;
use std::path::PathBuf;
use std::slice;
use std::fs::{self, File};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::Future;
use futures::sync::mpsc::{channel, Receiver};

use crate::error::*;

use hex_music_container::{Container, Configuration, error::Error as MusicError};
use hex_database::{Track, View};

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

    let output = Command::new("ffmpeg")
        .arg("-y").arg("-loglevel").arg("0").arg("-nostats")
        .arg("-ar").arg("48k")
        .arg("-ac").arg("2")
        .arg("-f").arg("s16le")
        .arg("-i").arg(file_path_out.to_str().unwrap())
        .arg("-metadata").arg(&format!("title=\"{}\"", track.title.clone().unwrap_or(track.key.to_string())))
        .arg("-metadata").arg(&format!("album=\"{}\"", track.album.clone().unwrap_or("Unknown".into())))
        .arg("-metadata").arg(&format!("author=\"{}\"", track.interpret.clone().unwrap_or("Unknown".into())))
        .arg("-metadata").arg(&format!("artist=\"{}\"", track.interpret.clone().unwrap_or("Unknown".into())))
        .arg("-metadata").arg(&format!("composer=\"{}\"", track.composer.clone().unwrap_or("Unknown".into())))
        .arg(converted_file.to_str().unwrap())
        .output().expect("Could not start ffmpeg!");

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);

        Err(Error::Ffmpeg(err.to_string()))
    } else {
        Ok((converted_file, track))
    }
}

pub struct State {
    pub recv: Receiver<DownloadProgress>
}

impl State {
    pub fn new(view: &View, tracks: Vec<Track>, data_path: PathBuf) -> State {
        let (sender, recv) = channel(200);

        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));

        for track in tracks {
            let data_path = data_path.clone();
            
            if !data_path.join("data").join(&track.key.to_string()).exists() {
                let pool = pool.clone();
                let mut s1 = sender.clone();
                let mut s2 = sender.clone();
                let c1 = counter.clone();
                let c2 = counter.clone();

                let res = view.ask_for_file(track.key.to_vec())
                    .and_then(move |x| {
                        pool.execute(move || {
                            let item = worker(track, data_path);
                            let cnt = c1.fetch_add(1, Ordering::Relaxed);
                            s2.try_send(DownloadProgress { result: item, num: cnt}).unwrap();
                        });

                        Ok(())
                    })
                    .or_else(move |_| {
                        let cnt = c2.fetch_add(1, Ordering::Relaxed);
                        s1.try_send(DownloadProgress { result: Err(Error::FileNotFound), num: cnt }).unwrap();

                        Ok(())
                    });

                tokio::spawn(res);
            } else {
                let mut sender = sender.clone();
                let counter = counter.clone();

                pool.execute(move || {
                    let item = worker(track, data_path);
                    let cnt = counter.fetch_add(1, Ordering::Relaxed);
                    sender.try_send(DownloadProgress { result: item, num: cnt}).unwrap();
                });
            }
        }

        State {
            recv
        }
    }
}
