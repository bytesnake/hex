<img align="left" src="/assets/github.png" width="220px"/>

#  Hex server - HTTP and websocket server, providing the frontend
_This crate is part of the [Hex](http://github.com/bytesnake/hex) project and bundles different libraries to a server application._

This is the main server application. It uses the `database`, `music_container` and `sync` crate
to manage the music and provides further routines to upload or download music from it.
It actually consists of three different servers. A HTTP server provides the frontend to
clients, the websocket server wraps function calls to the database and parses them and the sync
server synchronizes the database between peers. Each has its own port, as set in the
configuration, and the HTTP server as well as the sync server are disabled by default. To
enable them, they have to be in the configuration file:

```toml
host = "127.0.0.1"

[webserver]
path = "../frontend/build/"
port = 8081

[sync]
port = 8004
name = "Peer"
sync_all = true
```

and can then be passed as an argument. (e.g. `./target/release/hex_server conf.toml`)

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution
If you have any question or suggestion please just open an issue in Github. Feel free to comment on particular features or suggest crazy, funny ideas. I would anticipate if you can have some use of Hex and improve it at the same time.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
