use std::fmt::Debug;
use std::time::Instant;
use std::sync::atomic::AtomicIsize;
use std::sync::Arc;

use websocket::message::OwnedMessage;
use websocket::server::InvalidConnection;
use websocket::async::Server;

use tokio_core::reactor::{Handle, Core};
use futures::{Future, Sink, Stream};

use state::State;
use conf::Conf;

use hex_database::events::Action;

pub fn start(conf: Conf) {
	let mut core = Core::new().unwrap();
	let handle = core.handle();

	// bind to the server
    let addr = (conf.host, conf.server.port);
	let server = Server::bind(addr, &handle).unwrap();

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
            let conf_music = conf.music.clone();
            let token = Arc::new(AtomicIsize::new(-1));
            //let cards_2 = cards.clone();

            // accept the request to be a ws connection if it does
            let f = upgrade
                .use_protocol("rust-websocket")
                .accept()
                .and_then(move |(s,_)| {
                    let now = Instant::now();
                    let mut state = State::new(handle2, conf_music);

                    let (sink, stream) = s.split();

                    stream
                    //.take_while(|m| Ok(!m.is_close()))
                    .filter_map(move |m| {
                        match m {
                            OwnedMessage::Ping(p) => Some(OwnedMessage::Pong(p)),
                            OwnedMessage::Pong(_) => None,
                            OwnedMessage::Text(msg) => {
                                let msg = match state.process(addr.to_string(), msg, token.clone()) {
                                    Ok(msg) => msg,
                                    Err(_) => OwnedMessage::Text("Err(CouldNotParse)".into())
                                };

                                Some(msg)
                            },
                            OwnedMessage::Binary(data) => {
                                state.process_binary(&data);

                                Some(OwnedMessage::Text("{\"fn\": \"upload\"}".into()))
                            },
                            OwnedMessage::Close(_) => {
                                state.collection.add_event(Action::Connect(now.elapsed().as_secs() as f32).with_origin(addr.to_string())).unwrap();
                                println!("BLUB2");


                                Some(OwnedMessage::Close(None))
                            },
                            _ => Some(m)
                        }
                    })
                    .forward(sink)
                    .and_then(move |(_, sink)| {
                        println!("BLUB");
                        //println!("Disconnected: {}", now.elapsed().as_secs());
                        //hex_database::Collection::from_file(&conf_music.db_path)
                        //state.collection.add_event(Action::Connect(0.0).with_origin(addr.to_string())).unwrap();

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
