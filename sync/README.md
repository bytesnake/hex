<img align="left" src="/assets/github.png" width="220px"/>

#  Hex sync - replicates the database between peers
_This crate is part of the [Hex](http://github.com/bytesnake/hex) project and manages the database with respect to other peers_

Synchronize a music database with other peers in a local network. This crate implements 
peer discovery in an unknown network as well as peer-to-peer communication. On top of 
that a synchronize protocol replicates the database and manages all data. In the _partial_ mode
only the database is fully replicated and single audio files has to be requested in order to be
playable. This is for example useful in an Android application which allows to carry certain
playlists with you. In the _full_ mode everything is pulled in, useful in server applications.

# Example
```rust
// create a new peer with database path, data path, peer address, sync_everything
let (peer, chain) = Peer::new(
    Path::new("/opt/music/music.db"),
    Path::new("/opt/music/data/"),
    "127.0.0.1:8000".parse::<SocketAddr>(),
    false
);

// start the peer in a seperate thread
thread::spawn(|| tokio::run(chain));

// ask for a certain track to be available
peer.ask_for_track("<track_id>").wait();
```

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution
If you have any question or suggestion please just open an issue in Github. Feel free to comment on particular features or suggest crazy, funny ideas. I would anticipate if you can have some use of Hex and improve it at the same time.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
