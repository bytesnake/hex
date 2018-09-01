# Hex - a private music library

> "He was spending more nights now watching Hex trawl the invisible writings for any hints. In theory, because of the nature of L-space, absolutely everything was available to him, but that only meant that it was more or less impossible to find whatever it was you were looking for, which is the purpose of computers." - The Last Continent

Hex is a collection of crates with whom we're managing our music. It was born out of the desire to be independent from any music provider and to support music tags (which can be used like normal objects in the real world).

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

