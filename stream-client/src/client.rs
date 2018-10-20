use websocket::{ClientBuilder, OwnedMessage, self};
use std::net::TcpStream;
use rand::prelude::*;

use hex_server_protocol::{Request, RequestAction, Answer, PacketId};

pub struct Client {
    client: websocket::client::sync::Client<TcpStream>
}

pub fn gen_id() -> PacketId {
    random()
}

impl Client {
    pub fn new() -> Client {
        let client = ClientBuilder::new("ws://127.0.0.1:2794")
            .unwrap()
            .add_protocol("rust-websocket")
            .connect_insecure()
            .unwrap();

        println!("Connected to server!");

        Client {
            client: client
        }
    }

    pub fn send(&mut self, id: PacketId, msg: RequestAction) {
        let req = Request::new(id, msg);

        self.client.send_message(&OwnedMessage::Binary(req.to_buf().unwrap())).unwrap();
    }

    pub fn send_once(&mut self, msg: RequestAction) {
        self.send(gen_id(), msg);
    }

    pub fn recv(&mut self) -> Answer {
        let msg = self.client.recv_message().unwrap();

        match msg {
            OwnedMessage::Binary(buf) => Answer::try_from(&buf).unwrap(),
            _ => panic!("Got invalid message type!")
        }
    }
}
