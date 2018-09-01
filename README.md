# Hex - a private music library

Hex is a collection of crates with whom I'm managing my local music. It was born out of the desire to be independent from any music provider and to support music tags (which can be used like normal objects in the real world).

What are the goals?
 * having a music server running on a RaspberryPi
 * proper support for a text interface as well as a web interface
 * a substitute for objects (e.g. CD) with tags

From which parts is Hex made of?
 * (database) library - interface to a SQLite database
 * (music-container) library - codec for the music with Opus and Spherical Harmonics
 * (server) binary - a HTTP and websocket server providing the necessary calls
 * (frontend) website - nice GUI for music management
 * (local-client) binary - local management of the music collection without any server
 * (stream-client) binary - music playing system with support for tags in conjunction with a server

## Contribution

