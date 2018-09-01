extern crate byteorder;
extern crate opus;
extern crate futures;

pub mod error;
pub mod configuration;

use error::{Error, Result};
use std::io::{Seek, SeekFrom};
use std::fs::File;

use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};

use futures::sync::mpsc::Sender;

use opus::{Channels, Application};

pub use configuration::Configuration;

const RAW_BLOCK_SIZE: usize = 1920;

pub struct Container<T> {
    decoder: Vec<opus::Decoder>,
    inner: T,
    sh_order: u8,
    samples: u32,
    scales: Vec<f32>
}

impl<T> Container<T> 
    where T: ReadBytesExt + WriteBytesExt + Seek 
{
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

    pub fn empty(sh_order: u8, inner: T) -> Container<T> {
        Container::new(sh_order, 0, vec![1.0; (sh_order as usize+1)*(sh_order as usize+1)], inner)
    }

    pub fn load(mut inner: T) -> Result<Container<T>> {
        // read the version and SphericalHarmonic order fields
        let version = inner.read_u8().map_err(|err| Error::File(err))?;
        let sh_order = inner.read_u8().map_err(|err| Error::File(err))?;
       
        let samples = inner.read_u32::<LittleEndian>().map_err(|err| Error::File(err))?;

        let mut scales = Vec::new();
        for _ in 0..(sh_order+1)*(sh_order+1) {
            scales.push(inner.read_f32::<LittleEndian>().map_err(|err| Error::File(err))?);
        }

        if version != 1 {
            return Err(Error::CorruptedFile);
        }

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

    pub fn with_key(path: &str, key: &str) -> Result<Container<File>> {
        let file = File::open(format!("{}{}", path, key)).map_err(|err| Error::File(err))?;

        Container::load(file)
    }

    pub fn samples(&self) -> u32 {
        self.samples
    }

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

    pub fn num_harmonics(&self) -> u32 {
        (self.sh_order as u32 + 1)*(self.sh_order as u32 + 1)
    }

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

    pub fn save_pcm(conf: Configuration, pcm: &[i16], mut inner: T, mut progress: Option<Sender<f32>>) -> Result<Container<T>> {
        inner.write_u8(1).map_err(|err| Error::File(err))?;
        inner.write_u8(conf.sh_order()).map_err(|err| Error::File(err))?;
    
        let samples = pcm.len() as u32 / conf.num_channels();
        inner.write_u32::<LittleEndian>(samples).map_err(|err| Error::File(err))?;

        // scale the audio data to the max 16bit range
        let max_value = pcm.iter().map(|x| *x).max().ok_or(Error::InvalidRange)?;
        let scale = 32767.0 / max_value as f32;
        let pcm: Vec<i16> = pcm.iter().map(|x| (*x as f32 * scale) as i16).collect();

        // convert the raw audio channels to spherical harmonics
        let sh_codec = conf.codec();
        let harmonics: Vec<f32> = sh_codec.to_harmonics(&pcm)?;//vec![0f32; conf.num_harmonics() * samples];

        // determine scale factor for each harmonic to fit them into 16bit
        let mut sh_scales = vec![0.0; conf.num_harmonics() as usize];
        for sample in 0..samples as usize {
            for harmonic in 0..conf.num_harmonics() as usize {
                let val = harmonics[sample * conf.num_harmonics() as usize + harmonic].abs();

                if val > sh_scales[harmonic] {
                    sh_scales[harmonic] = val;
                }
            }
        }

        for scale in &mut sh_scales {
            if scale.abs() < std::f32::EPSILON {
                *scale = 1.0;
            }else {
                *scale = 32767.0 / *scale;
            }
        }

        // write the Spherical Harmonic scales to the file (the decoding process needs them to
        // restore the original scaling)
        for scale in &sh_scales {
            inner.write_f32::<LittleEndian>(*scale).map_err(|err| Error::File(err))?;
        }

        // compress with Opus and the proper scaling factor
        let mut opus_result: Vec<Vec<u8>> = (0..conf.num_harmonics()).map(|_| vec![0u8; 256]).collect();

        let mut encoders: Vec<opus::Encoder> = (0..conf.num_harmonics()).map(|_| {
            let mut encoder = opus::Encoder::new(48000, Channels::Mono, Application::Audio).unwrap();
            encoder.set_bitrate(opus::Bitrate::Max).unwrap();

            encoder
        }).collect();

        let steps = (samples as f32 / RAW_BLOCK_SIZE as f32).ceil() as usize;
        let mut nwritten = vec![0u16; conf.num_harmonics() as usize];

        for i in 0..steps {
            for j in 0..conf.num_harmonics() as usize {
                // convert each channel seperately with the corresponding scaling factor
                let mut source = vec![0i16; RAW_BLOCK_SIZE];

                // assert that we don't read after the buffer
                for k in 0..usize::min(RAW_BLOCK_SIZE, samples as usize - i * RAW_BLOCK_SIZE) {
                    source[k] = (harmonics[j + k * conf.num_harmonics() as usize + i * conf.num_harmonics() as usize * RAW_BLOCK_SIZE] * sh_scales[j]) as i16;
                }

                nwritten[j] = encoders[j].encode(&source, &mut opus_result[j]).map_err(|err| Error::Opus(err))? as u16;
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
                progress.try_send(i as f32 / steps as f32)
                    .map_err(|_| Error::SendFailed)?;
            }
        }
        if let Some(ref mut progress) = progress {
            progress.try_send(1.0)
                .map_err(|_| Error::SendFailed)?;
        }

        Ok(Container::new(conf.sh_order(), samples, sh_scales, inner))
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
