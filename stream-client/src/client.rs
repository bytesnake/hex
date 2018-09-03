use websocket::{ClientBuilder, OwnedMessage, self};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::net::TcpStream;
use std::thread;

use control;
use audio::AudioDevice;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Track {
    title: Option<String>,
    album: Option<String>,
    interpret: Option<String>,
    people: Option<String>,
    composer: Option<String>,
    fingerprint: String,
    pub key: String,
    pub duration: f64,
    favs_count: u32,
    channels: u32
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Playlist {
    pub key: String,
    pub title: String,
    desc: Option<String>,
    count: u32
}

#[derive(Deserialize, Serialize)]
pub struct Token {
    token: String,
    key: String,
    pos: usize,
    completion: f64
}

#[derive(Serialize)]
#[serde(tag = "fn")]
pub enum Outgoing {
    #[serde(rename="stream_next")]
    StreamNext {
        key: String
    },
    #[serde(rename="stream_end")]
    StreamEnd,
    #[serde(rename="stream_seek")]
    StreamSeek {
        pos: f64
    },
    #[serde(rename="get_token")]
    GetToken {
        token: String
    },
    #[serde(rename="insert_token")]
    InsertToken {
        token: Token
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Incoming {
    StreamNext,
    StreamSeek {
        pos: f64
    },
    StreamEnd,
    GetToken((Token, Playlist, Vec<Track>)),
    InsertToken
}

pub enum Packet {
    NextSong,
    PrevSong,
    NewToken(String),
    Shuffle
}

pub struct Client {
    sender: Sender<Packet>,
    thread: thread::JoinHandle<()>
}

impl Client {
    pub fn new(sender: Sender<control::Packet>, audio_device: AudioDevice) -> Client {
        let (sender, receiver) = channel();

        let client = ClientBuilder::new("127.0.0.1")
            .unwrap()
            .add_protocol("rust-websocket")
            .connect_insecure()
            .unwrap();

        println!("Connected to server!");

        let handle = thread::spawn(|| Self::run(client, receiver));

        Client {
            sender: sender,
            thread: handle
        }
    }

    pub fn sender(&self) -> Sender<Packet> {
        self.sender.clone()
    }

    pub fn run(client: websocket::sync::Client<TcpStream>, recv: Receiver<Packet>) {
        loop {
            
        }
    }
}
