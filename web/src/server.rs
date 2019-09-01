//! Websocket server implementation
//!
//! The websocket uses Tokio under the hood and manages a state for each connection. It also shares
//! the latest token to all clients and logs every events concerning connecting and disconnecting. 

use std::fmt::Debug;
use std::rc::Rc;
use std::cell::RefCell;
use std::path::PathBuf;

use websocket::WebSocketError;
use websocket::message::OwnedMessage;
use websocket::server::InvalidConnection;
use websocket::r#async::Server;

use tokio_core::reactor::{Handle, Core};
use futures::{Future, Sink, Stream, sync::mpsc::{Sender, channel}};

use crate::state::State;
use hex_conf::Conf;

use hex_server_protocol::{Answer, AnswerAction};
use hex_database::{Instance, GossipConf, TransitionAction};

/// Start the websocket server, supplied with a configuration
pub fn start(conf: Conf, path: PathBuf) {
	let mut core = Core::new().unwrap();
	let handle = core.handle();

	// bind to the server
    let addr = ("0.0.0.0", conf.server.port);

    let incoming = Server::bind(addr, &handle).unwrap().incoming();

    let mut gossip = GossipConf::new();
    
    if let Some(ref peer) = conf.peer {
        gossip = gossip.addr((conf.host, peer.port));
        gossip = gossip.id(peer.id());
        gossip = gossip.network_key(peer.network_key());
        gossip = gossip.contacts(peer.contacts.clone());
        gossip = gossip.discover(peer.discover);
    }

    let mut instance = Instance::from_file(&path.join("music.db"), gossip);

    let broadcasts: Rc<RefCell<Vec<Sender<TransitionAction>>>> = Rc::new(RefCell::new(Vec::new()));

    let tmp = broadcasts.clone();
    let c = instance.recv().for_each(|x| {
        let mut senders = tmp.borrow_mut();

        senders.retain(|x| !x.is_closed());

        for i in &mut *senders {
            if let Err(err) = i.try_send(x.clone()) {
                eprintln!("Got error: {}", err);
            }
        }

        Ok(())
    });

	// a stream of incoming connections
	let f = incoming
        .map(|x| Some(x))
        // we don't wanna save the stream if it drops
        .or_else(|InvalidConnection { error, .. }| {
            eprintln!("Error = {:?}", error);

            Ok(None)
        }).filter_map(|x| x)
        .for_each(|(upgrade, addr)| {
            info!("Got a connection from {} (to {})", addr, upgrade.uri());
            // check if it has the protocol we want
            if !upgrade.protocols().iter().any(|s| s == "rust-websocket") {
                // reject it if it doesn't
                spawn_future(upgrade.reject(), &handle);
                return Ok(());
            }

            let handle2 = handle.clone();
            let path_cpy = path.clone();
            let view = instance.view();
            let (s, r) = channel(1024);

            broadcasts.borrow_mut().push(s);

            // accept the request to be a ws connection if it does
            let f = upgrade
                .use_protocol("rust-websocket")
                .accept()
                .and_then(move |(s,_)| {
                    let mut state = State::new(handle2, &path_cpy, view);

                    let (sink, stream) = s.split();

                    let stream = stream.filter_map(move |m| {
                        match m {
                            OwnedMessage::Ping(p) => Some(OwnedMessage::Pong(p)),
                            OwnedMessage::Pong(_) => None,
                            OwnedMessage::Text(_) => Some(OwnedMessage::Text("Text not supported".into())),
                            OwnedMessage::Binary(data) => state.process(data).map(|x| OwnedMessage::Binary(x)),
                            OwnedMessage::Close(_) => {
                                info!("Client disconnected from {}", addr);
                                Some(OwnedMessage::Close(None))
                            }
                        }
                    })
                    .or_else(|e| {
                        eprintln!("Got websocket error = {:?}", e);

                        Ok(OwnedMessage::Close(None))
                    });

                    // forward transitions
                    let push = r.and_then(|x| {
                        Answer::new([0u32; 4], Ok(AnswerAction::Transition(x))).to_buf()
                            .map(|x| OwnedMessage::Binary(x))
                            .map_err(|_| ())
                    }).map_err(|_| WebSocketError::NoDataAvailable);

                    Stream::select(stream, push)
                        .forward(sink)
                        .and_then(move |(_, sink)| {
                            sink.send(OwnedMessage::Close(None))
                        })
                });

            spawn_future(f, &handle);

            Ok(())
        });

	core.run(Future::join(f, c)).unwrap();
}

fn spawn_future<F, I, E>(f: F, handle: &Handle)
	where F: Future<Item = I, Error = E> + 'static,
	      E: Debug
{
	handle.spawn(f.map_err(move |_| ())
	              .map(move |_| ()));
}
