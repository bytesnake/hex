use error::{ErrorKind, Result};
use failure::ResultExt;

use chromaprint::Chromaprint;
use std::str;
use serde_json;
use serde_json::Value;
use curl::easy::{Form,Easy};

pub fn get_hash(num_channel: u16, data: &[i16]) -> Result<String> {
    let mut ctx = Chromaprint::new();
    ctx.start(48000, num_channel as i32);

    ctx.feed(data);
    ctx.finish();

    Ok(ctx.fingerprint().ok_or(ErrorKind::AcousticID)?)
}

pub fn get_metadata(hash: &str, duration: u32) -> Result<String> {
    let mut dst = Vec::new();
    let mut easy = Easy::new();
    easy.url("https://api.acoustid.org/v2/lookup")
        .context(ErrorKind::AcousticIDMetadata)?;

    let mut form = Form::new();
    form.part("client").contents(b"GHPK6dMc-AY").add()
        .context(ErrorKind::AcousticIDMetadata)?;

    form.part("fingerprint").contents(hash.as_bytes()).add()
        .context(ErrorKind::AcousticIDMetadata)?;

    form.part("duration").contents(format!("{}", duration).as_bytes()).add()
        .context(ErrorKind::AcousticIDMetadata)?;

    form.part("meta").contents(b"recordings releasegroups").add()
        .context(ErrorKind::AcousticIDMetadata)?;

    easy.httppost(form)
        .context(ErrorKind::AcousticIDMetadata)?;
    
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|data| {
            dst.extend_from_slice(data);
            Ok(data.len())
        }).context(ErrorKind::AcousticIDMetadata)?;

        transfer.perform().context(ErrorKind::AcousticIDMetadata)?;
    }

    // convert result to a string
    let res_str = str::from_utf8(&dst).context(ErrorKind::AcousticIDMetadata)?;

    let v: Value = serde_json::from_str(res_str)
        .context(ErrorKind::AcousticIDMetadata)?;

    info!("{}", serde_json::to_string_pretty(&v).unwrap());

    match v["status"].as_str() {
        Some(status) if status == "ok" => {},
        Some(status) if status == "error" => {
            let err = match v["error"]["message"].as_str() {
                Some(msg) => format_err!("{}", msg).context(ErrorKind::AcousticIDMetadata).into(),
                None => ErrorKind::AcousticIDMetadata.into()
            };

            return Err(err);
        },
        _ => return Err(ErrorKind::AcousticIDMetadata.into())
    }

    if v["status"].as_str().unwrap() != "ok" {
        return Err(ErrorKind::AcousticIDMetadata.into());
    }

    Ok(serde_json::to_string(&v["results"]).unwrap())
}

