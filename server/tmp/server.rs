use std::fmt::Debug;
use std::sync::Arc;
use std::cell::RefCell;

use websocket::message::OwnedMessage;
use websocket::server::InvalidConnection;
use websocket::async::Server;

use tokio_core::reactor::{Handle, Core};
use futures::{Future, Sink, Stream};

use state::State;
use conf::Conf;

pub fn start(conf: Conf) {
	let mut core = Core::new().unwrap();
	let handle = core.handle();

	// bind to the server
    let addr = (conf.server.host.as_str(), conf.server.port);
	let server = Server::bind(addr, &handle).unwrap();

    //let card_key = Arc::new(RefCell::new(None));
    let card_key = Arc::new(RefCell::new(None));

	// time to build the server's future
	// this will be a struct containing everything the server is going to do

	// a stream of incoming connections
	let f = server.incoming()
        // we don't wanna save the stream if it drops
        .map_err(|InvalidConnection { error, .. }| error)
        .for_each(|(upgrade, addr)| {
            println!("Got a connection from: {}", addr);
            // check if it has the protocol we want
            if !upgrade.protocols().iter().any(|s| s == "rust-websocket") {
                // reject it if it doesn't
                spawn_future(upgrade.reject(), "Upgrade Rejection", &handle);
                return Ok(());
            }

            let handle2 = handle.clone();
            let card_key2 = card_key.clone();
            let conf_music = conf.music.clone();
            //let cards_2 = cards.clone();

            // accept the request to be a ws connection if it does
            let f = upgrade
                .use_protocol("rust-websocket")
                .accept()
                // send a greeting!
                //.and_then(|(s, _)| s.send(Message::text("Hello World!").into()))
                // simple echo server impl
                .and_then(|(s,_)| {
                    let mut state = State::new(handle2, conf_music);

                    let (sink, stream) = s.split();

                    stream
                    .take_while(|m| Ok(!m.is_close()))
                    .filter_map(move |m| {
                        match m {
                            OwnedMessage::Ping(p) => Some(OwnedMessage::Pong(p)),
                            OwnedMessage::Pong(_) => None,
                            OwnedMessage::Text(msg) => {
                                let mut key = card_key2.borrow_mut();
                                let msg = state.process(msg, &mut *key).unwrap();

                                Some(msg)
                            },
                            OwnedMessage::Binary(data) => {
                                state.process_binary(&data);

                                Some(OwnedMessage::Text("{\"fn\": \"upload\"}".into()))
                            },
                            _ => Some(m)
                        }
                    })
                    .forward(sink)
                    .and_then(|(_, sink)| {
                        sink.send(OwnedMessage::Close(None))
                    })
                });

            spawn_future(f, "Client Status", &handle);
            Ok(())
        });

    println!("Server is running!");

	core.run(f).unwrap();
}

fn spawn_future<F, I, E>(f: F, desc: &'static str, handle: &Handle)
	where F: Future<Item = I, Error = E> + 'static,
	      E: Debug
{
	handle.spawn(f.map_err(move |e| println!("{}: '{:?}'", desc, e))
	              .map(move |_| println!("{}: Finished.", desc)));
}
