//! Create fingerprint for a track and ask for metadata at acousticid.org

use chromaprint::Chromaprint;
use std::str;
use serde_json;
use serde_json::Value;
use curl::easy::{Form,Easy};

use crate::error::{Error, Result};

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

/*pub fn get_hash(fingerprint: &[i32]) -> i64 {
    let mut hasher = Sha256::new();

    let v_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            fingerprint.as_ptr() as *const u8,
            fingerprint.len() * std::mem::size_of::<i32>(),
        )
    };

    hasher.input(v_bytes);
    let a = hasher.result();
    (a[7] as i64) << 56 | (a[6] as i64) << 48 | (a[5] as i64) << 40 | (a[4] as i64) << 32 | 
        (a[3] as i64) << 24 | (a[2] as i64) << 16 | (a[1] as i64) << 8 | (a[0] as i64)
}*/

/// Fetch metadata from acousticid.org
///
/// With the fingerprint the acousticid server can be asked for the metadata. This function creates
/// a request and awaits its return.
///
///  * `hash` - Fingerprint of the audio file
///  * `duration` - Duration of the audio in millis
pub fn get_metadata(fingerprint: &[u32], duration: u32) -> Result<String> {
    let v_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            fingerprint.as_ptr() as *const u8,
            fingerprint.len() * std::mem::size_of::<i32>(),
        )
    };

    let fingerprint = base64::encode(v_bytes);

    let mut dst = Vec::new();
    let mut easy = Easy::new();
    easy.url("https://api.acoustid.org/v2/lookup")
        .map_err(|_| Error::AcousticIDMetadata)?;

    let mut form = Form::new();
    form.part("client").contents(b"sepmArwuV3").add()
        .map_err(|_| Error::AcousticIDMetadata)?;

    form.part("fingerprint").contents(fingerprint.as_bytes()).add()
        .map_err(|_| Error::AcousticIDMetadata)?;

    form.part("duration").contents(duration.to_string().as_bytes()).add()
        .map_err(|_| Error::AcousticIDMetadata)?;

    form.part("meta").contents(b"recordings releasegroups").add()
        .map_err(|_| Error::AcousticIDMetadata)?;

    easy.httppost(form)
        .map_err(|_| Error::AcousticIDMetadata)?;
    
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|data| {
            dst.extend_from_slice(data);
            Ok(data.len())
        }).map_err(|_| Error::AcousticIDMetadata)?;

        transfer.perform().map_err(|_| Error::AcousticIDMetadata)?;
    }

    // convert result to a string
    let res_str = str::from_utf8(&dst).map_err(|_| Error::AcousticIDMetadata)?;

    let v: Value = serde_json::from_str(res_str)
        .map_err(|_| Error::AcousticIDMetadata)?;

    //info!("{}", serde_json::to_string_pretty(&v).unwrap());

    match v["status"].as_str() {
        Some(status) if status == "ok" => {},
        Some(status) if status == "error" => {
            let err = match v["error"]["message"].as_str() {
                Some(msg) => Error::AcousticIDResponse(msg.into()),
                None => Error::AcousticIDMetadata
            };

            return Err(err);
        },
        _ => return Err(Error::AcousticIDMetadata)
    }

    if v["status"].as_str().unwrap() != "ok" {
        return Err(Error::AcousticIDMetadata);
    }

    serde_json::to_string(&v["results"])
        .map_err(|_| Error::AcousticIDMetadata)
}

