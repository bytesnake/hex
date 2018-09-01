#[derive(Debug, Clone)]
pub struct Track {
    pub title: Option<String>,
    pub album: Option<String>,
    pub interpret: Option<String>,
    pub people: Option<String>,
    pub composer: Option<String>,
    pub fingerprint: String,
    pub key: String,
    pub duration: f64,
    pub favs_count: u32,
    pub channels: u32 
}

impl Track {
    pub fn empty(fingerprint: String, key: String, duration: f64, channels: u32) -> Track {
        Track {
            title: None,
            album: None,
            interpret: None,
            people: None,
            composer: None,
            fingerprint: fingerprint,
            key: key,
            duration: duration,
            favs_count: 0,
            channels: channels
        }
    }
}

#[derive(Debug)]
pub struct Playlist {
    pub key: String,
    pub title: String,
    pub desc: Option<String>,
    pub count: u32
}

#[derive(Debug)]
pub struct Token {
    pub token: String,
    pub key: String,
    pub pos: u32,
    pub completion: f64
}
