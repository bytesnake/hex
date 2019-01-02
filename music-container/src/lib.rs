//! Spatial encoding and compressing with Opus of raw audio data
//!
//! This crate takes raw audio data with a configuration describing the channel arrangement and
//! converts it into Hex own audio format. It compresses it thereby with Opus and encodes it in an
//! Spherical Harmonic representation, supporting spatial audio. The decoding step can
//! convert the audio file to any loudspeaker configuration, reconstructing the spatial audio
//! field.
//!
//! ## File format
//!
//! The file format is the following:
//! |       |    1    |     1    |     4    | (order+1)**2 * 4 | (order+1)**2 * samples * 2 |
//! |-------|---------|----------|----------|------------------|----------------------------|
//! | field | version | sh order | samples  | scales ..        | audio data ...             |
//!
extern crate byteorder;
extern crate opus;
extern crate futures;

pub mod error;
pub mod configuration;

use std::path::Path;
use std::io::{Seek, SeekFrom};
use std::fs::File;

use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use futures::sync::mpsc::Sender;
use opus::{Channels, Application};

use error::{Error, Result};
pub use configuration::Configuration;

/// Size of a single raw audio block
const RAW_BLOCK_SIZE: usize = 1920;

/// Represents an open audio file
pub struct Container<T> {
    /// Each SH channel needs its own decoder
    decoder: Vec<opus::Decoder>,
    /// The underlying audio file
    inner: T,
    /// Spherical Harmonic order (describing the spatial resolution)
    sh_order: u8,
    /// Number of samples in the audio file
    samples: u32,
    /// SH scales for each SH channel
    scales: Vec<f32>
}

impl<T> Container<T> 
    where T: ReadBytesExt + WriteBytesExt + Seek 
{
    /// Creates a new `Container`
    pub fn new(sh_order: u8, samples: u32, scales: Vec<f32>, inner: T) -> Container<T> {
        let mut ct = Container {
            decoder: (0..(sh_order+1)*(sh_order+1)).map(|_| opus::Decoder::new(48000, Channels::Mono).unwrap()).collect(),
            sh_order: sh_order,
            samples: samples,
            scales: scales,
            inner: inner 
        };

        ct.seek_to_data();

        ct
    }

    /// Creates a new empty container
    pub fn empty(sh_order: u8, inner: T) -> Container<T> {
        Container::new(sh_order, 0, vec![1.0; (sh_order as usize+1)*(sh_order as usize+1)], inner)
    }

    /// Load a file and parses the header
    pub fn load(mut inner: T) -> Result<Container<T>> {
        // read the version and SphericalHarmonic order fields
        let version = inner.read_u8().map_err(|err| Error::File(err))?;
        let sh_order = inner.read_u8().map_err(|err| Error::File(err))?;
       
        // read in the number of samples
        let samples = inner.read_u32::<LittleEndian>().map_err(|err| Error::File(err))?;

        // read in all SH scales
        let mut scales = Vec::new();
        for _ in 0..(sh_order+1)*(sh_order+1) {
            scales.push(inner.read_f32::<LittleEndian>().map_err(|err| Error::File(err))?);
        }

        // Only this version is supported at the moment
        if version != 1 {
            return Err(Error::CorruptedFile);
        }

        // There will never be a order larger than 6
        if sh_order > 6 {
            return Err(Error::CorruptedFile);
        }

        
        //let header_size = 6 + 4 * (sh_order as u64 + 1) * (sh_order as u64 + 1);
        //let mut rem = inner.seek(SeekFrom::End(0)).unwrap() -  header_size;
        //inner.seek(SeekFrom::Start(header_size + 4));

        //println!("Open file version: {}, SHOrder: {}, Samples: {}, Rem: {}", version, sh_order, samples, rem);

        //println!("Compression ratio {}", samples as f32 * 2.0 / rem as f32);

        Ok(Container::new(sh_order, samples, scales, inner))
    }

    /// Open a audio file from a certain path
    pub fn with_key(path: &Path, key: &str) -> Result<Container<File>> {
        let file = File::open(path.join(key)).map_err(|err| Error::File(err))?;

        Container::load(file)
    }

    /// Get number of samples
    pub fn samples(&self) -> u32 {
        self.samples
    }

    /// Seek to the beginning of the data section
    pub fn seek_to_data(&mut self) {
        self.inner.seek(SeekFrom::Start(6 + 4 * (self.sh_order as u64 + 1) * (self.sh_order as u64 + 1))).unwrap();
    }

    /*pub fn check_samplesize(&mut self) -> Result<()> {
        self.seek_to_data();

        let mut buf = vec![];
        self.inner.read_to_end(&mut buf).unwrap();

        let samples = self.decoder[0].get_nb_samples(&buf[50..])
            .map_err(|err| Error::Opus(err))?;

        println!("Announced: {}, Measured: {}", buf.len(), samples);

        if self.samples as usize != samples {
            Err(Error::CorruptedFile)
        } else {
            Ok(())
        }
    }
    */

    /// Seek to a certain sample in the underlying memory
    pub fn seek_to_sample(&mut self, sample: u32) {
        self.seek_to_data();

        let mut pos = 0;
        while pos + RAW_BLOCK_SIZE < sample as usize {
            let mut skip = 0i64;
            for _ in 0..(self.sh_order as usize +1)*(self.sh_order as usize +1) {
                skip += self.inner.read_u8().unwrap() as i64;
            }

            self.inner.seek(SeekFrom::Current(skip)).unwrap();

            pos += RAW_BLOCK_SIZE;
        }
    }

    /// Number of Spherical Harmonic channels
    pub fn num_harmonics(&self) -> u32 {
        (self.sh_order as u32 + 1)*(self.sh_order as u32 + 1)
    }

    /// Decode a single raw audio buffer with a certain loudspeaker configuration
    pub fn next_packet(&mut self, conf: Configuration) -> Result<Vec<i16>> {
        let sizes: Vec<Result<u8>> = (0..self.num_harmonics()).map(|_| {
            self.inner.read_u8().map_err(|_| Error::ReachedEnd)
        }).collect();
        let codec = conf.codec();

        let mut buf = vec![0u8; 256];
        let mut harmonic_unscaled = vec![0i16; RAW_BLOCK_SIZE];
        let mut harmonics = vec![0f32; RAW_BLOCK_SIZE * (self.sh_order as usize + 1) * (self.sh_order as usize + 1)];

        let mut i = 0;
        for size in sizes {
            let size = size.map_err(|_| Error::ReachedEnd)?;

            let nread = self.inner.read(&mut buf[0..size as usize])
                .map_err(|_| Error::ReachedEnd)?;

            if nread != size as usize {
                return Err(Error::CorruptedFile);
            }

            //println!("Size: {}", size);
            let nwritten = self.decoder[i].decode(&buf[0..nread], &mut harmonic_unscaled, false).unwrap();

            let harmonic_scaled: Vec<f32> = harmonic_unscaled.iter().map(|x| *x as f32 / self.scales[i]).collect();

            for j in 0..RAW_BLOCK_SIZE {
                harmonics[j * self.num_harmonics() as usize + i] = harmonic_scaled[j];
            }

            if nwritten != RAW_BLOCK_SIZE {
                return Err(Error::CorruptedFile);
            }

            i += 1;
        }

        codec.to_channels(&harmonics, self.sh_order)
    }

    /// Converts raw audio with loudspeaker configuration to a new `Container`
    ///
    /// The `progress` field can be used to connect a channel to the convesion process and get live
    /// updates of the progress.
    pub fn save_pcm(conf: Configuration, mut pcm: Vec<i16>, mut inner: T, mut progress: Option<Sender<f32>>) -> Result<Container<T>> {
        inner.write_u8(1).map_err(|err| Error::File(err))?;
        inner.write_u8(conf.sh_order()).map_err(|err| Error::File(err))?;
    
        // fill the audio signal encoded as channels up to multiple of RAW_BLOCK_SIZE
        let mut samples = pcm.len() / conf.num_channels() as usize;
        let rem = RAW_BLOCK_SIZE - samples % RAW_BLOCK_SIZE;
        pcm.extend(vec![0; rem * conf.num_channels()]);
        samples += rem;

        inner.write_u32::<LittleEndian>(samples as u32).map_err(|err| Error::File(err))?;

        // scale the audio data to the max 16bit range
        let max_value = pcm.iter().map(|x| *x).max().ok_or(Error::InvalidRange)?;
        let scale = 32767.0 / max_value as f32;

        for val in pcm.iter_mut() {
            *val = (*val as f32 * scale) as i16;
        }

        // find the scales
        let sh_codec = conf.codec();
        let scales = sh_codec.scales(&pcm)?;

        for scale in &scales {
            inner.write_f32::<LittleEndian>(*scale).map_err(|err| Error::File(err))?;
        }

        // the audio signal encoded in spherical harmonics
        let mut harmonics = vec![0i16; RAW_BLOCK_SIZE * conf.num_harmonics()];
        let mut opus_result: Vec<Vec<u8>> = (0..conf.num_harmonics()).map(|_| vec![0u8; 256]).collect();

        let mut encoders: Vec<opus::Encoder> = (0..conf.num_harmonics()).map(|_| {
            let mut encoder = opus::Encoder::new(48000, Channels::Mono, Application::Audio).unwrap();
            encoder.set_bitrate(opus::Bitrate::Max).unwrap();

            encoder
        }).collect();

        let mut nwritten = vec![0u16; conf.num_harmonics() as usize];

        for i in 0..samples / RAW_BLOCK_SIZE {
            let channels = &pcm[i*RAW_BLOCK_SIZE*conf.num_channels() .. (i+1)*RAW_BLOCK_SIZE*conf.num_channels()];
            sh_codec.to_harmonics(&scales, &channels, &mut harmonics);

            for j in 0..conf.num_harmonics() as usize {
                nwritten[j] = encoders[j].encode(&harmonics[j*RAW_BLOCK_SIZE..(j+1)*RAW_BLOCK_SIZE], &mut opus_result[j]).map_err(|err| Error::Opus(err))? as u16;
            }

            //println!("Loss: {:?}, Bitrate: {:?}, Bandwidth: {:?}, Written: {:?}", encoders[0].get_packet_loss_perc().unwrap(), encoders[0].get_bitrate().unwrap(), encoders[0].get_bandwidth().unwrap(), nwritten);

            // write each harmonic 
            for c in 0..conf.num_harmonics() as usize {
                inner.write_u8(nwritten[c] as u8).map_err(|err| Error::File(err))?;
            }

            for c in 0..conf.num_harmonics() as usize {
                inner.write(&opus_result[c][0..nwritten[c] as usize]).map_err(|err| Error::File(err))?;
            }

            if let Some(ref mut progress) = progress {
                progress.try_send(i as f32 / (samples / RAW_BLOCK_SIZE) as f32)
                    .map_err(|_| Error::SendFailed)?;
            }
        }
        if let Some(ref mut progress) = progress {
            progress.try_send(1.0)
                .map_err(|_| Error::SendFailed)?;
        }

        Ok(Container::new(conf.sh_order(), samples as u32, scales, inner))
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{Read, Write};
    use std::slice;

    use super::{Container, Configuration, RAW_BLOCK_SIZE};

    #[test]
    fn convert_pcm() {
        let mut pcm_file = File::open("assets/crazy.raw").unwrap();
        let mut pcm = vec![];

        pcm_file.read_to_end(&mut pcm).unwrap();

        let pcm2: &[i16] = unsafe {
            slice::from_raw_parts(
                pcm.as_ptr() as *const i16,
                pcm.len() / 2
            )
        };

        let opus_file = File::create("assets/crazy.opus").unwrap();
        
        Container::save_pcm(Configuration::Stereo, pcm2, opus_file).unwrap();
    }

    #[test]
    fn read_opus() {
        let mut opus_file = File::open("assets/crazy.opus").unwrap();

        let mut file = Container::from_file(opus_file).unwrap();

        //file.seek_to_sample(121 * 1000 * 1000);

        file.seek_to_data();

        let mut out_file = File::create("assets/crazy2.raw").unwrap();
        //let mut buf = vec![0i16; RAW_BLOCK_SIZE];
        let mut samples = 0;

        while samples < file.samples {    
            //file.next_packet(&mut buf);
            let buf = file.next_packet(Configuration::Stereo).unwrap();

            let pcm: &[u8] = unsafe {
                slice::from_raw_parts(
                    buf.as_ptr() as *const u8,
                    buf.len() * 2
                )
            };

            out_file.write(&pcm).unwrap();

            samples += RAW_BLOCK_SIZE as u32;
        }

    }
}
