use std::str;
use std::thread;
use std::path::PathBuf;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command;
use std::ffi::OsStr;

use futures::IntoFuture;
use futures::sync::oneshot::{channel, Sender, Receiver};
use hex_database::Track;
use hex_music_container::{Container, Configuration};

use crate::error::*;

/// Calculate a fingerprint to lookup music
///
/// This function takes a raw audio and number of channels and calculates the corresponding
/// fingerprint, strongly connected to the content.
///
///  * `num_channel`- Number of channel in `data`
///  * `data` - Raw audio data with succeeding channels
pub fn get_fingerprint(raw_path: PathBuf) -> Result<Vec<u32>> {
    let cmd = Command::new("fpcalc")
        .arg(raw_path.to_str().unwrap())
        .arg("-rate").arg("48000")
        .arg("-channels").arg("2")
        .arg("-format").arg("s16le")
        .arg("-plain").arg("-raw")
        .output().expect("Could not start ffmpeg!");

    let out = str::from_utf8(&cmd.stdout)
        .map_err(|_| Error::AcousticID)?;

    out.trim().split(",").map(|x| x.parse::<u32>().map_err(|_| Error::AcousticID)).collect()
}

fn worker(sender: Sender<Track>, file_name: String, samples: Vec<u8>, data_path: PathBuf) -> Result<()> {
    let encoded_path = data_path.join("download").join(&file_name);
    let raw_path = data_path.join("download").join(&file_name).with_extension("pcm");

    let mut file = File::create(&encoded_path)
        .map_err(|x| Error::Io(x))?;

    file.write(&samples)
        .map_err(|x| Error::Io(x))?;

    Command::new("ffmpeg")
        .arg("-y")//.arg("-loglevel").arg("panic").arg("hide_banner")
        .arg("-i").arg(encoded_path.to_str().unwrap())
        .arg("-ar").arg("48k")
        .arg("-ac").arg("2")
        .arg("-f").arg("s16le")
        .arg("-acodec").arg("pcm_s16le")
        .arg(raw_path.to_str().unwrap())
        .spawn().expect("Could not start ffmpeg!").wait().unwrap();

    let mut samples = Vec::new();
    let mut file = File::open(&raw_path)
        .map_err(|x| Error::Io(x))?; 

    file.read_to_end(&mut samples)
        .map_err(|x| Error::Io(x))?;

    let duration = samples.len() as f64 / 48000.0 / 2.0;

    let samples: &[i16] = unsafe { ::std::slice::from_raw_parts(samples.as_ptr() as *const i16, samples.len() / 2) };


    let fingerprint = get_fingerprint(raw_path)?;

    let mut track = Track::empty(fingerprint, duration.into());

    let file = File::create(data_path.join("data").join(track.key.to_path())).unwrap();

    // TODO realtime
    Container::save_pcm(Configuration::Stereo, samples.to_vec(), file, None)
        .map_err(|err| Error::MusicContainer(err))?;

    match encoded_path.extension().and_then(OsStr::to_str) {
        Some("mp3") => {
            if let Ok(metadata) = mp3_metadata::read_from_file(encoded_path) {
                if let Some(metadata) = metadata.tag {

                    track.title = Some(metadata.title);
                    track.album = Some(metadata.album);
                    track.composer = Some(metadata.artist.clone());
                    track.interpret = Some(metadata.artist);
                } else {
                    for tag in metadata.optional_info {
                        if let Some(title) = tag.title {
                            track.title = Some(title);
                        }
                        if let Some(album) = tag.album_movie_show {
                            track.album = Some(album);
                        }
                        if let Some(performer) = tag.performers.get(0) {
                            track.composer = Some(performer.clone());
                            track.interpret = Some(performer.clone());
                        }
                    }
                }
            }
        }
        _ => {}
    }

    sender.send(track).map_err(|_| Error::ChannelFailed)
}

pub struct Upload {
    recv: Receiver<Track>
}

impl Upload {
    pub fn new(file_name: String, content: Vec<u8>, data_path: PathBuf) -> Upload {
        let (sender, recv) = channel();

        thread::spawn(move || {
            worker(sender, file_name, content, data_path)
                .map_err(|e| {eprintln!("{:?}", e); e})
                .map(|_| ())
        });

        Upload { recv }
    }
}

impl IntoFuture for Upload {
    type Future = Receiver<Track>;
    type Item = Track;
    type Error = futures::sync::oneshot::Canceled;

    fn into_future(self) -> Self::Future {
        self.recv
    }
}
