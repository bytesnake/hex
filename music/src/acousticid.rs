use error::{Error, Result};

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

    ctx.fingerprint().ok_or(Error::Internal)
}


/*#[derive(Deserialize, Debug)]
pub struct Artist {
    pub id: String,
    pub name: String
}

#[derive(Deserialize, Debug)]
pub struct ReleaseGroup {
    pub artists: Option<Vec<Artist>>,
    pub title: String,
    pub id: String
}

#[derive(Deserialize, Debug)]
pub struct Recording {
    pub artists: Option<Vec<Artist>>,
    pub releasegroups: Option<Vec<ReleaseGroup>>,
    pub id: String,
    pub title: Option<String>

}

#[derive(Deserialize, Debug)]
pub struct MusicEntity {
    pub id: String,
    pub score: f64,
    pub recordings: Option<Vec<Recording>>
}

#[derive(Deserialize, Debug)]
pub struct Tracks(Vec<MusicEntity>);
*/

//impl Tracks {
    pub fn get_metadata(hash: &str, duration: u32) -> Result<String> {
        let mut dst = Vec::new();
        let mut easy = Easy::new();
        easy.url("https://api.acoustid.org/v2/lookup")
            .map_err(|_| Error::Internal)?;

        let mut form = Form::new();
        form.part("client").contents(b"-G4yxla2DdAI").add().map_err(|_| Error::Internal)?;
        form.part("fingerprint").contents(hash.as_bytes()).add().map_err(|_| Error::Internal)?;
        form.part("duration").contents(format!("{}", duration).as_bytes()).add().map_err(|_| Error::Internal)?;
        form.part("meta").contents(b"recordings releasegroups").add().map_err(|_| Error::Internal)?;
        easy.httppost(form).map_err(|_| Error::Internal)?;
        
        {
            let mut transfer = easy.transfer();
            transfer.write_function(|data| {
                dst.extend_from_slice(data);
                Ok(data.len())
            }).map_err(|_| Error::RequestAborted)?;

            transfer.perform().map_err(|_| Error::RequestAborted)?;
        }

        // convert result to a string
        let res_str = str::from_utf8(&dst).map_err(|_| Error::Parsing)?;

        let v: Value = serde_json::from_str(res_str)
            .map_err(|_| Error::Parsing)?;

        info!("{}", serde_json::to_string_pretty(&v).unwrap());

        match v["status"].as_str() {
            Some(status) if status == "ok" => {},
            Some(status) if status == "error" => {
                let err = match v["error"]["message"].as_str() {
                    Some(msg) => Error::AcousticID(msg.into()),
                    None => Error::Parsing
                };

                return Err(err);
            },
            _ => return Err(Error::Parsing)
        }

        if v["status"].as_str().unwrap() != "ok" {
            return Err(Error::Parsing);
        }

        /*let recs: Tracks = serde_json::from_value(v["results"].clone())
            .map_err(|_| Error::Parsing)?;
*/

        println!("ADJSLDJSALK");

        Ok(serde_json::to_string(&v["results"]).unwrap())
    }

/*
    /// Returns a pair of title name and id to the corresponding recording
    pub fn get_titles(&self) -> Vec<(String, String)> {
        let mut res = Vec::new();
        
        for track in &self.0 {
            let ref id = track.id;

            if let Some(ref recordings) = track.recordings {
                for record in recordings {
                    if let Some(ref title) = record.title {
                        res.push((title.clone(), id.clone()));
                    }
                }
            }
        }

        res
    }

    pub fn get_albums(&self, id: &str, title: &str) -> Vec<String> {
        let mut res = Vec::new();

        // the ID is unique, therefore only one recording
        if let Some(track) = self.0.iter().filter(|x| x.id == id).next() {
            if let Some(ref recordings) = track.recordings {
                for record in recordings {
                    if let Some(ref title_curr) = record.title {
                        if title == title_curr {
                            if let Some(ref groups) = record.releasegroups {
                                for album in groups {
                                    res.push(album.title.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        res
    }

    pub fn get_artists(&self, id: &str, title: &str) -> Vec<String> {
        let mut res = Vec::new();

        // the ID is unique, therefore only one recording
        if let Some(track) = self.0.iter().filter(|x| x.id == id).next() {
            if let Some(ref recordings) = track.recordings {
                for record in recordings {
                    if let Some(ref title_curr) = record.title {
                        if title == title_curr {
                            if let Some(ref artists) = record.artists {
                                for artist in artists {
                                    res.push(artist.name.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        res
    }
}*/
