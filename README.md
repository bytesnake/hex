<img align="left" src="assets/github.png" width="220px"/>

#  Hex - a private music library
> "He was spending more nights now watching Hex trawl the invisible writings for any hints. In theory, because of the nature of L-space, absolutely everything was available to him, but that only meant that it was more or less impossible to find whatever it was you were looking for, which is the purpose of computers."
>
> &mdash; <cite>Terry Pratchett (in _The Last Continent_)</cite>

Hex is a collection of crates which can store, manage, tokenise and play music. It was born out of the desire to be independent from any music provider and to support music tags (real world objects like CDs representing a playlist). The project is written in Rust and at the moment running on two platforms, a music server and player.

What are the goals?
 * having a music server running on a Raspberry Pi
 * proper support for a text interface as well as a web interface
 * a substitute for objects (e.g. CD) with tags

From which parts is Hex made of?
 * [database](database/) library - interface to a SQLite database
 * [music-container](music-container/) library - codec for the music with Opus and Spherical Harmonics
 * [server](server) binary - a HTTP and websocket server providing the necessary calls
 * [frontend](frontend) website - nice GUI for music management
 * [local-client](local-client) binary - local management of the music collection without any server
 * [stream-client](stream-client) binary - music playing system with support for tags in conjunction with a server
 * [nightly-worker](nightly-worker) binary - summarise each day and perform some kind of cleanup

How are these crates interacting?

The Hex project is all about music and its very important for us to have a acessible and easy user experience. For a developer this means that the project is chunked into useful components. The server plays the role of providing the music to every client with help of the database and music-container crates. The database crate defines all objects like _Playlist_, _Track_, _Token_, etc. and provides useful functions to manage them in a SQLite database. The _music-container_ converts raw audio to the Hex specific audio format. Two important points are that is uses the Opus codec to achieve good compression levels and saves the audio in a Spherical Harmonic format (though only minimal support at the moment, but extendable and backward compatible). With help of those libraries the server offers JSON calls to modify the database, play and swallow music. It can also provide the _frontend_ with help of a HTTP server. The _frontend_ connects to the websocket server and gives a nice overview and some tools to manage the music. The second streaming client (working with websockets) is the _stream-client_ which supports Tokens and runs on a small ARM chip with four buttons and the MFRC522 reader. The _local-client_ is a handy tool to manage the database without the graphical burden of a frontend. It can add music, change metadata and list information about Hex. As a local client it can only be used on the same computer as the server.

<img align="left" src="assets/zyklop_confused.png" width="190px"/>

## Future directions
 * improve frontend with people, playlist image and download
 * full SH support
 * more MUSIC!

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution
If you have any question or suggestion please just open an issue in Github. Feel free to comment on particular features or suggest crazy, funny ideas. I would anticipate if you can have some use of Hex and improve it at the same time.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
