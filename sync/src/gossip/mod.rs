mod protocol;

use std::sync::{Mutex, Arc};
use std::net::SocketAddr;
use std::collections::HashMap;

use futures::{Async, Stream, task};
use futures::sync::mpsc::{Receiver, Sender, channel};
use tokio::io;
use tokio::net::{TcpListener, TcpStream, Incoming};

use self::protocol::{Packet, Peer, ResolvePeers, PeerCodecWrite};

/// Identification of a peer. For now this is a unique name.
pub type PeerId = String;

/// Contains information about the whereabouts of a peer
///
/// The identity as well as the connection to a peer are stored here. They are
/// telling us how to reach out for a peer and how we should encrypt data for him.
/// For now this contains only the name of a peer, but later on it can be a
/// public key (as part of a keyring) and a unique identification. (for example
/// the hash of the public key)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerPresence {
    pub id: PeerId,
    addr: SocketAddr,
    writer: Option<usize>
}

// TODO pub struct PeerHabits;

pub struct GossipPush {
    peers: Mutex<HashMap<PeerId, PeerCodecWrite<TcpStream>>>
}

impl GossipPush {
    pub fn new() -> GossipPush {
        GossipPush { peers: Mutex::new(HashMap::new()) }
    }

    pub fn add_peer(&self, id: &PeerId, writer: PeerCodecWrite<TcpStream>) -> usize {
        let mut peers = self.peers.lock().unwrap();
        let len = peers.keys().len();
        peers.insert(id.clone(), writer);

        return len;
    }

    pub fn write_packet(&self, id: &PeerId, data: Packet) -> Result<(), io::Error> {
        let mut peers = self.peers.lock().unwrap();

        let writer = peers.get_mut(id).ok_or(io::ErrorKind::NotFound)?;
        writer.buffer(data);
        writer.poll_flush().map(|_| ())
    }
    pub fn write(&self, id: &PeerId, buf: Vec<u8>) -> Result<(), io::Error> {
        self.write_packet(id, Packet::Push(buf))
    }

    pub fn push(&self, buf: Vec<u8>) -> Result<(), io::Error> {
        let mut peers = self.peers.lock().unwrap();
        let packet = Packet::Push(buf);

        for peer in peers.values_mut() {
            peer.buffer(packet.clone());
            peer.poll_flush().map(|_| ())?;
        }

        Ok(())
    }
}

/// Implements the peer sampling and data dissemination
///
/// It consists of four parts. First a channel to which connected peers are hooked up. They
/// will send packets through the PeerCodec. Second an incoming field to accept new peers asking
/// for a connection. Third a stream of emerging connections which are not fully established. And
/// forth a log of existing connections to peer.
pub struct Gossip {
    myself: PeerPresence,
    recv: Receiver<(PeerId, Packet)>,
    sender: Sender<(PeerId, Packet)>,
    books: HashMap<PeerId, PeerPresence>,
    writer: Arc<GossipPush>,
    resolve: ResolvePeers,
    incoming: Incoming
}

impl Gossip {
    pub fn new(addr: SocketAddr, contact: Option<SocketAddr>, id: PeerId) -> Gossip {
        let (sender, receiver) = channel(1024);
        let listener = TcpListener::bind(&addr).unwrap();

        let myself = PeerPresence {
            id: id,
            addr: listener.local_addr().unwrap(),
            writer: None
        };

        let peers = match contact {
            Some(addr) => {
                vec![Peer::connect(&addr, myself.clone())]
            },
            None => Vec::new()
        };

        println!("Gossip: Start server with addr {:?}", addr);

        Gossip {
            myself: myself,
            recv: receiver,
            sender: sender,
            books: HashMap::new(),
            incoming: listener.incoming(),
            resolve: ResolvePeers::new(peers),
            writer: Arc::new(GossipPush::new())
        }
    }

    pub fn writer(&self) -> Arc<GossipPush> {
        self.writer.clone()
    }
}

/// Create a new stream, managing the gossip protocol
impl Stream for Gossip {
    type Item = (PeerId, Vec<u8>);
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        // first look for newly arriving peers and await a Join message
        match self.incoming.poll() {
            Ok(Async::Ready(Some(socket))) => {
                self.resolve.add_peer(Peer::wait_for_join(socket, self.myself.clone()));
            },
            Err(err) => {
                println!("Listener err: {:?}", err);

                return Err(err);
            },
            _ => {}
        }

        // poll all connecting peers
        //
        match self.resolve.poll() {
            Ok(Async::Ready(Some((reader, mut writer, mut presence)))) => {
                //println!("Gossip: connection established from {} to {}", self.myself.id, presence.id);

                // ask for other peers if this is our contact
                if self.books.is_empty() {
                    writer.buffer(Packet::GetPeers(None));
                    writer.poll_flush().unwrap();
                }

                if self.books.contains_key(&presence.id) || self.myself.id == presence.id {
                    println!("Got already existing id: {}", presence.id);

                    writer.shutdown();
                } else {

                    // empty a new log entry for our peer
                    let idx = self.writer.add_peer(&presence.id, writer);
                    presence.writer = Some(idx);

                    // hook up the packet output to us
                    reader.redirect_to(self.sender.clone(), presence.id.clone(), task::current());
                    self.books.insert(presence.id.clone(), presence.clone());

                    // the connection is established
                    return Ok(Async::Ready(Some((presence.id, Vec::new()))));
                }


            },
            _ => {}
        }

        // now try to get a new packet from the hooked peers
        let res = self.recv.poll();
        let (id, packet) = try_ready!(res.map_err(|_| io::ErrorKind::Other)).unwrap();
        
        // and process it with some logic
        match packet {
            Packet::GetPeers(None) => {
                let mut list: Vec<PeerPresence> = self.books.values().cloned()
                    .filter_map(|mut x| {
                        if x.id != id {
                            x.writer = None;
                            return Some(x);
                        }
                        
                        return None;
                    }).collect();

                self.writer.write_packet(&id, Packet::GetPeers(Some(list))).unwrap();
            },
            Packet::GetPeers(Some(peers)) => {
                for presence in peers {
                    if !self.books.contains_key(&presence.id) && !self.resolve.has_peer(&presence.id) {
                        println!("Gossip: Add peer {} in {}", presence.id, self.myself.id);
                        self.resolve.add_peer(Peer::connect(&presence.addr, self.myself.clone()));
                    }
                }
            }
            Packet::Push(data) => {
                // the peer has send us a new block of data, forward it
                return Ok(Async::Ready(Some((id, data))));
            },
            Packet::Close => {
                self.books.remove(&id);

                //println!("Gossip: Connection closed to {}", id);
            },
            _ => {}
        }

        return Ok(Async::NotReady);
    }
}
