use std::fs::File;
use std::process::Command;
use std::path::Path;
use std::io::Write;
use std::mem;
use uuid::Uuid;

use error::{Result, ErrorKind, MyError};
use failure::ResultExt;

use acousticid;
use database::Track;

use opus;
use opus::{Channels, Application};

use hound::WavReader;

pub struct AudioFile {
    key: String,
    duration: f64,
    opus_data: Vec<u8>,
    fingerprint: String
}

impl AudioFile {
    pub fn new(data: &[u8], format: &str) -> Result<AudioFile> {
        // convert to wave file
        let file_name = format!("/tmp/temp_music.{}", format);
        let mut file = File::create(&file_name)
            .context(ErrorKind::Conversion)?;

        file.write_all(data)
            .context(ErrorKind::Conversion)?;

        file.sync_all()
            .context(ErrorKind::Conversion)?;

        let mut output = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i").arg(&file_name)
            .arg("-ar").arg("48000")
            .arg("/tmp/temp_music_new.wav")
            .spawn()
            .context(ErrorKind::Conversion)?;

        output.wait()
            .context(ErrorKind::Conversion)?;

        AudioFile::from_wav_48k("/tmp/temp_music_new.wav")
    }

    /// a wave audio file with 48k sample rate is assumed
    pub fn from_wav_48k(path: &str) -> Result<AudioFile> {
        // read the whole wave file
        let mut reader = WavReader::open(path)
            .context(ErrorKind::Conversion)?;

        let samples = reader.samples::<i16>().map(|x| x.unwrap()).collect::<Vec<i16>>();

        // use the metadata section to determine sample rate, number of channel and duration in
        // seconds
        let sample_rate = reader.spec().sample_rate as f64;
        let num_channel = reader.spec().channels;
        let duration = reader.duration() as f64 / sample_rate as f64;

        debug!("Open file {} ({} samples) with sample rate {} and {} channels", path, samples.len(),sample_rate, num_channel);

        AudioFile::from_raw_48k(samples, duration, num_channel)
    }

    pub fn from_raw_48k(samples: Vec<i16>, duration: f64, num_channel: u16) -> Result<AudioFile> {
        // calculate the acousticid of the file
        let fingerprint = acousticid::get_hash(num_channel, &samples)?;
        let key = Uuid::new_v4();

        debug!("Calculated fingerprint: {}", fingerprint);
        debug!("The corresponding key is {}", key);

        if Path::new(&format!("/home/lorenz/.music/{}", key)).exists() {
            return Err(format_err!("File with key {} already exists!", key).context(ErrorKind::Conversion).into());
        }

        // now convert to the opus file format
        let channel = match num_channel {
            1 => Channels::Mono,
            _ => Channels::Stereo // TODO: more than two channels support
        };
            
        let mut opus_data: Vec<u8> = Vec::new();
        let mut tmp = vec![0u8; 4000];

        let mut encoder = opus::Encoder::new(48000, channel, Application::Audio)
            .context(ErrorKind::Conversion)?;
        
        for i in samples.chunks(1920) {
            let nbytes: usize = {
                if i.len() < 1920 {
                    let mut filled_up_buf = vec![0i16; 1920];
                    filled_up_buf[0..i.len()].copy_from_slice(i);

                    encoder.encode(&filled_up_buf, &mut tmp)
                        .context(ErrorKind::Conversion)?
                } else {
                    encoder.encode(&i, &mut tmp)
                        .context(ErrorKind::Conversion)?
                }
            };

            //println!("Opus frame size: {}", nbytes);

            tmp.truncate(nbytes);

            let nbytes_raw: [u8; 4] = unsafe { mem::transmute((nbytes as u32).to_be()) };

            opus_data.extend_from_slice(&nbytes_raw);
            opus_data.extend_from_slice(&tmp);
        }

        info!("Size: {}", opus_data.len());
        info!("Duration: {}", duration);

        Ok(AudioFile { key: key.simple().to_string(), duration: duration, fingerprint: fingerprint, opus_data: opus_data })
    }
    
    pub fn to_db(&mut self) -> Result<Track> {
        let mut file = File::create(&format!("/home/lorenz/.music/{}", self.key))
            .context(ErrorKind::Conversion)?;

        file.write_all(&self.opus_data)
            .context(ErrorKind::Conversion)?;

        Ok(Track::empty(&self.key, &self.fingerprint, self.duration))
    }
}
