use uuid::Uuid;

use serde_json::{self, value::Value};
use websocket::{ClientBuilder, OwnedMessage, self};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::net::TcpStream;
use std::thread;

use audio::AudioDevice;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Track {
    pub title: Option<String>,
    pub album: Option<String>,
    pub interpret: Option<String>,
    pub people: Option<String>,
    pub composer: Option<String>,
    pub key: String,
    pub duration: f64,
    pub favs_count: u32,
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
    pub pos: usize,
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
        sample: u32
    },
    #[serde(rename="get_token")]
    GetToken {
        token: u32
    },
    #[serde(rename="insert_token")]
    InsertToken {
        token: Token
    },
    #[serde(rename="vote_for_track")]
    VoteForTrack {
        key: String
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
#[serde(untagged)]
pub enum Incoming {
    #[serde(rename = "stream_next")]
    StreamNext,
    #[serde(rename = "stream_seek")]
    StreamSeek {
        pos: u32
    },
    #[serde(rename = "stream_end")]
    StreamEnd,
    #[serde(rename = "get_token")]
    GetToken((Token, Playlist, Vec<Track>)),
    #[serde(rename = "insert_token")]
    InsertToken,
    #[serde(rename = "vote_for_track")]
    VoteForTrack,
    Buffer(Vec<u8>)
}

impl Incoming {
    pub fn deserialize(buf: String) -> Result<Incoming, Error> {
        let mut wrapper: IncomingWrapper = serde_json::from_str(&buf).expect("Failed to deserialize Incoming Wrapper!");

        println!("{}", buf);

        let mut res: Result<Value, Error> = serde_json::from_value(wrapper.payload.clone()).unwrap();

        res.map(|x| {
            match wrapper.fnc.as_ref() {
                "stream_next" => Incoming::StreamNext,
                "stream_end" => Incoming::StreamEnd,
                "insert_token" => Incoming::InsertToken,
                "vote_for_track" => Incoming::VoteForTrack,
                _ => serde_json::from_value(x).unwrap()
            }
        })
    }
}

#[derive(Deserialize, Debug)]
pub enum Error {
    #[serde(rename = "MusicContainer(ReachedEnd)")]
    EndOfStream
}

#[derive(Deserialize, Debug)]
pub struct IncomingWrapper {
    id: String,
    #[serde(rename = "fn")]
    fnc: String,
    payload: Value
}

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

    pub fn send_once(&mut self, msg: Outgoing) {
        self.send(Uuid::new_v4(), msg);
    }

    pub fn recv(&mut self) -> Result<Incoming, Error> {
        let msg = self.client.recv_message().unwrap();

        match msg {
            OwnedMessage::Text(msg) => Incoming::deserialize(msg),
            OwnedMessage::Binary(buf) => Ok(Incoming::Buffer(buf)),
            _ => panic!("Got invalid message type!")
        }
    }
}
