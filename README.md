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

## Contribution

<img align="left" src="assets/zyklop_trouble.png" width="220px"/>

## Problems

