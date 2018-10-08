//! Loudspeaker configuration
//!
//! This module is used in `hex_music_container` to describe the loudspeaker configuration of raw
//! audio. It can also be disabled in case the raw audio is already in SH format.

use error::{Error, Result};

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
    pub fn num_harmonics(&self) -> u32 {
        let order = self.sh_order() as u32;

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
    pub fn num_channels(&self) -> u32 {
        match *self {
            Configuration::Omnidirectional => 1,
            Configuration::Stereo => 2,
            Configuration::Binaural => 2,
            Configuration::SphericalHarmonics(x) => (x as u32 +1)*(x as u32 +1)
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
    /// Converts raw audio to SH representation
    pub fn to_harmonics(&self, channels: &[i16]) -> Result<Vec<f32>> {
        let num_channels = self.conf.num_channels() as usize;
        let num_harmonics = self.conf.num_harmonics() as usize;
        let samples = channels.len() / num_channels;
            

        if channels.len() % self.conf.num_channels() as usize != 0 {
            return Err(Error::InvalidSize);
        }

        let mut harmonics = vec![0.0; samples * num_harmonics];

        let mut i = 0;
        for sample in channels.chunks(num_channels) {
            match self.conf {
                Configuration::Omnidirectional => {
                    harmonics[i] = sample[0] as f32 * 0.2820948;
                },
                Configuration::Stereo => {
                    harmonics[i] = (sample[0] as f32 + sample[1] as f32) * 0.5 * 0.2820948;
                    harmonics[i+1] = (sample[0] as f32 - sample[1] as f32) * 0.5 * 0.3454941;
                    harmonics[i+2] = 0.0;
                    harmonics[i+3] = (sample[1] as f32 - sample[0] as f32) * 0.5 * 0.3454941;
                },
                _ => return Err(Error::NotSupported)
            }

            i += self.conf.num_harmonics() as usize;
        }

        Ok(harmonics)
    }

    /// Converts SH representation to loudspeaker dependent representation
    pub fn to_channels(&self, harmonics: &[f32], from_harmonics: u8) -> Result<Vec<i16>> {
        let num_channels = self.conf.num_channels() as usize;
        let num_from_harmonics = (from_harmonics as usize + 1) * (from_harmonics as usize + 1);
        let samples = harmonics.len() / num_from_harmonics;
        
        let mut channels = vec![0; samples * num_channels];

        let mut i = 0;
        for sample in harmonics.chunks(num_from_harmonics) {
            match self.conf {
                Configuration::Omnidirectional => {
                    channels[i] = (sample[0] * 3.5449077) as i16;
                },
                Configuration::Stereo => {
                    if sample.len() == 1 {
                        channels[i] = (1.7724538 * sample[0]) as i16;
                        channels[i+1] = (1.7724538 * sample[0]) as i16;
                    } else {
                        channels[i] = (1.4472025 * sample[1] + 1.7724538 * sample[0]) as i16;
                        channels[i+1] = (1.4472025 * sample[3] + 1.7724538 * sample[0]) as i16;
                    }
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
    fn test_ambi() {
        let conf = Configuration::Stereo;
        let codec = conf.codec();

        let buf: Vec<i16> = (-100..100).collect();
        let buf_harmonics = codec.to_harmonics(&buf).unwrap();
        let buf_channels = codec.to_channels(&buf_harmonics, 2).unwrap();

        println!("{:?}", buf_channels);

    }
}
