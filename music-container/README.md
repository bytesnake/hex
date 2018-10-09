<img align="left" src="/assets/github.png" width="220px"/>

#  Hex music-container - compress and encode audio
_This crate is part of the [Hex](http://github.com/bytesnake/hex) project and used to compress with Opus and encode to a loudspeaker independent format._

This library exists because the music data has to be stored in some way. One, simple, approach would be to use the MP3 format with stereo encoding all the time and just upmix or downmix different channel ordering. This would be sufficient for many cases, but not compatible to spatial audio or binaural reproduction. This crate followes therefore a different path. It encodes the raw audio to a source independent representation with Spherical Harmonics and compresses each SH channel with the Opus codec. Later on the audio file can be decoded again to any loudspeaker arrangement. (in limitation of the spatial resolution) Most of the SH codec calculation is missing for now, but can be added later on in a compatible way.

## Example
```rust
extern crate hex_music_container;

use hex_music_container::{Configuration, Container};

fn main() {
    let container = Container::with_key("/tmp/music/data", "<key>").unwrap();

    let block = container.next_packet(Configuration::Stereo);
    println!("Got block n = {}", block.len());
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
