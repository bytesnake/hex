use uuid::Uuid;

use serde_json;
use websocket::{ClientBuilder, OwnedMessage, self};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::net::TcpStream;
use std::thread;

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

#[derive(Deserialize, Serialize, Debug)]
pub struct Token {
    token: u32,
    key: String,
    pos: usize,
    completion: f64
}

impl Token {
    pub fn with_playlist(token: u32, key: &str) -> Token {
        Token {
            token: token,
            key: key.into(),
            pos: 0,
            completion: 0.0
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "fn")]
pub enum Outgoing {
    #[serde(rename="stream_next")]
    StreamNext {
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>
    },
    #[serde(rename="stream_end")]
    StreamEnd,
    #[serde(rename="stream_seek")]
    StreamSeek {
        pos: u32
    },
    #[serde(rename="get_token")]
    GetToken {
        token: u32
    },
    #[serde(rename="insert_token")]
    InsertToken {
        token: Token
    }
}

#[derive(Serialize)]
pub struct OutgoingWrapper {
    id: String,
    msg: Outgoing
}

impl Outgoing {
    pub fn serialize(self, id: Uuid) -> String {
        let wrapper = OutgoingWrapper {
            id: id.hyphenated().to_string(),
            msg: self
        };

        serde_json::to_string(&wrapper).expect("Failed to serialize Outgoing struct!")
    }
}

#[derive(Deserialize, Debug)]
//#[serde(untagged)]
pub enum Incoming {
    StreamNext,
    StreamSeek {
        pos: u32
    },
    StreamEnd,
    GetToken((Token, Playlist, Vec<Track>)),
    InsertToken
}

impl Incoming {
    pub fn deserialize(buf: String) -> Result<Incoming, Error> {
        serde_json::from_str(&buf).expect("Failed to deserialize Incoming!")
    }
}

#[derive(Deserialize, Debug)]
pub enum Error {
}

#[derive(Deserialize, Debug)]
pub struct IncomingWrapper((String, Result<Incoming, Error>));

pub struct Client {
    client: websocket::client::sync::Client<TcpStream>
}

impl Client {
    pub fn new() -> Client {
        let client = ClientBuilder::new("ws://192.168.1.2:2794")
            .unwrap()
            .add_protocol("rust-websocket")
            .connect_insecure()
            .unwrap();

        println!("Connected to server!");

        Client {
            client: client
        }
    }

    pub fn send(&mut self, id: Uuid, msg: Outgoing) {
        self.client.send_message(&OwnedMessage::Text(msg.serialize(id))).unwrap();
    }
}
