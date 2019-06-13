use std::io::Write;
use std::path::PathBuf;
use std::slice;
use std::thread;
use std::fs::{self, File};
use std::process::Command;
use std::rc::Rc;
use std::cell::RefCell;

use futures::{IntoFuture, Future, Stream};
use futures::sync::mpsc::{channel, Sender, Receiver};

use crate::error::*;

use hex_music_container::{Container, Configuration, error::Error as MusicError};
use hex_database::Track;

pub struct DownloadProgress {
    pub path: PathBuf,
    pub track: Track,
    pub num: usize
}


fn worker(mut sender: Sender<DownloadProgress>, tracks: Vec<Track>, num_channel: u32, data_path: PathBuf) -> Result<()> {
    let download_path = data_path.join("download");
    println!("start working at {:?}", data_path);

    for i in 0..tracks.len() {
        println!("processing {}", i);
        let file_path = data_path.join("data").join(&tracks[i].key.to_string());

        let file_path_out = download_path
            .join(&tracks[i].interpret.clone().unwrap_or("unknown".into()))
            .join(&tracks[i].album.clone().unwrap_or("unknown".into()));

        fs::create_dir_all(&file_path_out).unwrap();

        let file_path_out = file_path_out.join(tracks[i].title.clone().unwrap_or(tracks[i].key.to_string()));

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
        println!("convert end");

        let converted_file = file_path_out.with_extension("mp3");

        Command::new("ffmpeg")
            .arg("-y")
            .arg("-ar").arg("48k")
            .arg("-ac").arg("2")
            .arg("-f").arg("s16le")
            .arg("-i").arg(file_path_out.to_str().unwrap())
            .arg(converted_file.to_str().unwrap())
            .spawn().expect("Could not start ffmpeg!").wait().unwrap();

        println!("ffmpeg end");

        sender.try_send(DownloadProgress { path: converted_file, track: tracks[i].clone() , num: i}).unwrap();
    }

    Ok(())
}

pub struct State {
    thread: thread::JoinHandle<Result<()>>,
    pub recv: Receiver<DownloadProgress>
}

impl State {
    pub fn new(tracks: Vec<Track>, num_channel: u32, data_path: PathBuf) -> State {
        let (sender, recv) = channel(10);

        let thread = thread::spawn(move || {
            worker(sender, tracks, num_channel, data_path)
                .map_err(|e| {eprintln!("{:?}", e); e})
                .map(|_| ())
        });

        State {
            thread: thread,
            recv
        }
    }
}
