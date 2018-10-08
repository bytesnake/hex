<img align="left" src="/assets/github.png" width="220px"/>

#  Hex database - manage metadata of tracks, playlists and tokens
_This crate is part of the [Hex](http://github.com/bytesnake/hex) project and is used in various implementations like `server`, `local-client` and `sync`_
What would a music collection be without any metadata? Very boring. The `hex_database` crate manages all metadata concerning tracks, playlists and tokens, and provides the infrastructure for logs and summarization of each day. Furthermore it contains a parser for search queries and an easy-to-use interface encapsulating important functions like adding tracks, favouriting tracks or creating tokens.

```rust
extern crate hex_database;

use hex_database::{Collection, SearchQuery};

fn main() {
    // open a connection to the database
    let path = Path::new("/tmp/music.db");
    let conn = Connection::from_file(&path);

    // query all tracks and playlists
    let tracks = conn.get_tracks();
    let playlists = conn.get_playlists();

    println!("The database contains {} tracks and {} playlists", tracks.len(), playlists.len());

    // search for the track 'Love Me' in 'Catch A Bird'
    let query = SearchQuery::new("title:Love Me album:Catch A Bird").unwrap();
    let mut stmt = conn.search_prepare(query).unwrap();
    
    let track = conn.search(&mut stmt).next().expect("Track not found!");
    println!("Searched: {:#?}", track);

    // nice song!
    conn.vote_for_track(track.key).unwrap();

    // create a new playlist with the previous track
    let playlist = conn.create_playlist("Songs from Marbert Rocel", None).unwrap();
    conn.add_to_playlist(track.key, &playlist.title).unwrap();
}
```

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution
If you have any question or suggestion please just open an issue in Github. Feel free to comment on particular features or suggest crazy, funny ideas. I would anticipate if you can have some use of Hex and improve it at the same time.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
