//! Loudspeaker configuration
//!
//! This module is used in `hex_music_container` to describe the loudspeaker configuration of raw
//! audio. It can also be disabled in case the raw audio is already in SH format.

use crate::error::{Error, Result};

/// A loudspeaker configuration
#[derive(Clone)]
pub enum Configuration {
    /// One channel describes sound coming from all directions at the same time
    Omnidirectional,
    /// Two channels contains sound coming from the left and right direction
    Stereo,
    /// Binaural coding (only useful in decoding, not encoding)
    Binaural,
    /// Spherical Harmonic format
    SphericalHarmonics(u8)
}

impl Configuration {
    /// Get the number of SH channels
    pub fn num_harmonics(&self) -> usize {
        let order = self.sh_order() as usize;

        (order+1)*(order+1)
    }

    /// Get the SH order
    pub fn sh_order(&self) -> u8 {
        match *self {
            Configuration::Omnidirectional => 0,
            Configuration::Stereo => 1,
            Configuration::Binaural => 1,
            Configuration::SphericalHarmonics(x) => x
        }
    }

    /// Get the number of loudspeaker channels
    pub fn num_channels(&self) -> usize {
        match *self {
            Configuration::Omnidirectional => 1,
            Configuration::Stereo => 2,
            Configuration::Binaural => 2,
            Configuration::SphericalHarmonics(x) => (x as usize +1)*(x as usize +1)
        }
    }

    /// Create a codec from this configuration
    pub fn codec(&self) -> Codec {
        Codec {
            conf: self.clone()
        }
    }
}

/// This codec converts a block of raw audio to a loudspeaker independent audio representation
pub struct Codec {
    conf: Configuration
}

impl Codec {
    pub fn scales(&self, channels: &[i16]) -> Result<Vec<f32>> {
        let num_channels = self.conf.num_channels() as usize;
        let num_harmonics = self.conf.num_harmonics() as usize;

        let mut scales = vec![std::f32::MAX; num_harmonics];
            
        for sample in channels.chunks(num_channels) {
            match self.conf {
                Configuration::Omnidirectional => {
                    if sample[0] as f32 * 0.2820948 > scales[0] {
                        scales[0] = sample[0] as f32 * 0.2820948;
                    }
                },
                Configuration::Stereo => {
                    let val = 32767.0 / ((sample[0] as f32 + sample[1] as f32) * 0.2820948).abs();
                    if val < scales[0] {
                        scales[0] = val;
                    }

                    let val = 32767.0 / ((sample[0] as f32 - sample[1] as f32) * 0.3454941).abs();
                    if val < scales[1] {
                        scales[1] = val;
                    }

                    let val = 32767.0 / ((sample[1] as f32 - sample[0] as f32) * 0.3454941).abs();
                    if val < scales[3] {
                        scales[3] = val;
                    }

                    scales[2] = 1.0;
                },
                _ => return Err(Error::NotSupported)
            }
        }
        
        Ok(scales)
    }

    /// Converts raw audio to SH representation
    pub fn to_harmonics(&self, scales: &[f32], channels: &[i16], harmonics: &mut [i16]) {
        let num_channels = self.conf.num_channels() as usize;
        //let num_harmonics = self.conf.num_harmonics() as usize;

        // calculate the number of samples per block
        let block_length = channels.len() / num_channels;

        let first = 1.0 / (4.0 * std::f64::consts::PI).sqrt();
        let secon = (3.0 / 8.0 / std::f64::consts::PI).sqrt();

        //println!("{} {} {}", num_channels, num_harmonics, block_length);
        let mut i = 0;
        for sample in channels.chunks(num_channels) {
            match self.conf {
                Configuration::Omnidirectional => {
                    harmonics[i] = (sample[0] as f64 * first / scales[0] as f64) as i16;
                },
                Configuration::Stereo => {
                    harmonics[i] = ((sample[0] as f64 * 0.7 + sample[1] as f64 * 0.7) * first * scales[0] as f64).round() as i16;
                    harmonics[i + block_length] = ((sample[0] as f64 * 0.7 - sample[1] as f64 * 0.7) * secon * scales[1] as f64).round() as i16;
                    harmonics[i + block_length*2] = 0;
                    harmonics[i + block_length*3] = ((sample[1] as f64 * 0.7 - sample[0] as f64 * 0.7) * secon * scales[3] as f64).round() as i16;
                },
                _ => {}
            }

            i += 1;
        }
    }

    /// Converts SH representation to loudspeaker dependent representation
    pub fn to_channels(&self, scales: &[f32], harmonics: &[i16], from_harmonics: u8) -> Result<Vec<i16>> {
        let num_channels = self.conf.num_channels() as usize;
        let num_from_harmonics = (from_harmonics as usize + 1) * (from_harmonics as usize + 1);
        let samples = harmonics.len() / num_from_harmonics;
        
        let mut channels = vec![0; samples * num_channels];
        let sample = harmonics;

        let first = (4.0 * std::f64::consts::PI / 4.0).sqrt();
        let secon = (8.0 * std::f64::consts::PI / 3.0 / 4.0).sqrt();

        let mut i = 0;
        for j in 0..samples {
            match self.conf {
                Configuration::Omnidirectional => {
                    channels[i] = ((sample[j] as f64) / scales[0] as f64 * first) as i16;
                },
                Configuration::Stereo => {
                    channels[i] = (secon * (sample[j + samples] as f64 / scales[1] as f64) + first * (sample[j] as f64 / scales[0] as f64)).round() as i16;
                    channels[i+1] = (secon * (sample[j + samples * 3] as f64 / scales[3] as f64) + first * (sample[j] as f64 / scales[0] as f64)).round() as i16;
                },
                _ => return Err(Error::NotSupported)
            }

            i += self.conf.num_channels() as usize;
        }

        Ok(channels)
    }
}


#[cfg(test)]
mod test {
    use super::Configuration;

    #[test]
    fn test_linear_sequence_stereo() {
        let conf = Configuration::Stereo;
        let codec = conf.codec();

        let buf: Vec<i16> = (-100..100).collect();

        let scales = codec.scales(&buf).unwrap();

        let mut buf_harmonics = vec![0; buf.len() * 2];
        codec.to_harmonics(&scales, &buf, &mut buf_harmonics);

        let buf_channels = codec.to_channels(&scales, &buf_harmonics, 1).unwrap();

        assert_eq!(buf_channels, buf);
    }

    #[test]
    fn test_sine_sequence_stereo() {
        let conf = Configuration::Stereo;
        let codec = conf.codec();

        let buf: Vec<i16> = (0..48000).map(|x| {
            let arg = x as f64 / 48000.0 * 20.0;
            let val = arg.sin();

            (val * 4000.0) as i16
        }).collect();

        let scales = codec.scales(&buf).unwrap();

        let mut buf_harmonics = vec![0; buf.len() * 2];

        codec.to_harmonics(&scales, &buf, &mut buf_harmonics);
        let buf_channels = codec.to_channels(&scales, &buf_harmonics, 1).unwrap();

        assert_eq!(buf_channels, buf);
    }
}
