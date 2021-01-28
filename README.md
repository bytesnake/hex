# Hex Version 2.0

This is a complete rewrite of Hex. It aims to be simpler and better maintainable, because peer to peer systems are not suited for all situations. The architecture contains two basic blocks:
 * Zyklop - interfacing with external ridges and giving feedback
 * CLI - maintaining the music library

The communication between music player and clients is done completely over SSH with the Git protocol. This simplifies the architecture a lot. The remotes are basically different zyklopes with shared music.
