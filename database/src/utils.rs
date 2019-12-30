use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::str;

use crate::error::*;

pub fn fingerprint_from_file(num_channels: u16, raw_path: &Path) -> Result<Vec<u32>> {                                                                                                                              
    let cmd = Command::new("fpcalc")
        .arg(raw_path.to_str().unwrap())
        .arg("-rate").arg("48000")
        .arg("-channels").arg(num_channels.to_string())
        .arg("-format").arg("s16le")
        .arg("-plain").arg("-raw")
        .output().expect("Could not start ffmpeg!");

    let out = str::from_utf8(&cmd.stdout)
        .map_err(|_| Error::AcousticId)?;

    out.trim().split(",").map(|x| x.parse::<u32>().map_err(|_| Error::AcousticId)).collect()
}

pub fn get_fingerprint(num_channels: u16, data: &[i16]) -> Result<Vec<u32>> {
    let mut file = tempfile::NamedTempFile::new().unwrap();

   let v_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            data.as_ptr() as *const u8,
            data.len() * std::mem::size_of::<i16>(),
        )
    };

   file.write(v_bytes).unwrap();

   fingerprint_from_file(num_channels, file.path())
}
