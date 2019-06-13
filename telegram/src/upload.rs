use std::thread;
use std::path::PathBuf;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command;

use futures::{Future, IntoFuture};
use futures::sync::oneshot::{channel, Sender, Receiver};
use hex_database::Track;
use hex_music_container::{Container, Configuration};

use chromaprint::Chromaprint;

use crate::error::*;

/// Calculate a fingerprint to lookup music
///
/// This function takes a raw audio and number of channels and calculates the corresponding
/// fingerprint, strongly connected to the content.
///
///  * `num_channel`- Number of channel in `data`
///  * `data` - Raw audio data with succeeding channels
pub fn get_fingerprint(num_channel: u16, data: &[i16]) -> Result<Vec<u32>> {
    let mut ctx = Chromaprint::new();
    ctx.start(48000, num_channel as i32);

    ctx.feed(data);
    ctx.finish();

    ctx.raw_fingerprint().ok_or(Error::AcousticID)
        .map(|x| x.into_iter().map(|x| x as u32).collect())
}

fn worker(sender: Sender<Track>, file_name: String, samples: Vec<u8>, data_path: PathBuf) -> Result<()> {
    let encoded_path = data_path.join("download").join(&file_name);
    let raw_path = data_path.join("download").join(&file_name).with_extension("pcm");

    let mut file = File::create(&encoded_path)
        .map_err(|x| Error::Io(x))?;

    file.write(&samples)
        .map_err(|x| Error::Io(x))?;

    Command::new("ffmpeg")
        .arg("-y")
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

    let duration = (samples.len() as f64 / 48000.0 / 2.0);

    let samples: &[i16] = unsafe { ::std::slice::from_raw_parts(samples.as_ptr() as *const i16, samples.len() / 2) };


    let fingerprint = get_fingerprint(2, &samples)?;
    let mut track = Track::empty(fingerprint, duration.into());

    let file = File::create(data_path.join("data").join(track.key.to_path())).unwrap();

    // TODO realtime
    Container::save_pcm(Configuration::Stereo, samples.to_vec(), file, None)
        .map_err(|err| Error::MusicContainer(err))?;

    track.title = Some(file_name);

    sender.send(track).map_err(|_| Error::ChannelFailed)
}

pub struct Upload {
    recv: Receiver<Track>
}

impl Upload {
    pub fn new(file_name: String, content: Vec<u8>, data_path: PathBuf) -> Upload {
        let (sender, recv) = channel();

        let thread = thread::spawn(move || {
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
