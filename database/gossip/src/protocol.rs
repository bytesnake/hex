//! Handle packet encoding and peer ACK

use std::net::SocketAddr;
use std::mem;
use std::fmt::Debug;
use std::io::ErrorKind;
use std::collections::HashMap;

use futures::task::Task;
use futures::sync::mpsc::Sender;
use futures::stream::{futures_unordered, FuturesUnordered};
use tokio::prelude::*;
use tokio::{self, io, io::ReadHalf, io::WriteHalf};
use tokio::net::{TcpStream, tcp::ConnectFuture};
use bytes::{BytesMut, BufMut};
use bincode::{deserialize, serialize};
use ring::{aead, rand, rand::SecureRandom, aead::Nonce, aead::Aad};

use crate::{PeerId, PeerPresence, Error, Result};
use crate::transition::{Transition, TransitionKey};

/// The network key will be shared between all peers and contains 
/// a 256bit key, encrypting and signing every transition send through the network
pub type NetworkKey = [u8; 32];


/// Peer-to-Peer message
/// 
/// The protocol is not very complex. After establishing a connection
/// every peer should send a Join message as a handshake. The peers
/// can then be requested with the GetPeers message. A push transmits
/// a new transition of the database state
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Packet {
    /// We ask to join in the network with a identity and our tips of the data graph
    Join(PeerPresence, Vec<Transition>, Vec<TransitionKey>),
    /// Ask for the current vector of peers
    GetPeers(Option<Vec<PeerPresence>>),
    /// Push a new packet into the network with reference to received transitions
    Push(Transition),
    Other(Vec<u8>),
    Close
}

/// List of peers to be resolved
pub struct ResolvePeers {
    awaiting: FuturesUnordered<Peer>,
    ids: HashMap<PeerId, ()>
}

impl ResolvePeers {
    pub fn new(peers: Vec<Peer>) -> ResolvePeers {
        ResolvePeers {
            awaiting: futures_unordered(peers),
            ids: HashMap::new()
        }
    }

    pub fn add_peer(&mut self, peer: Peer) {
        self.awaiting.push(peer);
    }

    pub fn poll(&mut self) -> Poll<Option<(PeerCodecRead<TcpStream>, PeerCodecWrite<TcpStream>, PeerPresence, Vec<Transition>, Vec<TransitionKey>)>, io::Error> {
        if let Some((read, write, presence, transitions, missing)) = try_ready!(self.awaiting.poll()) {
            self.ids.insert(presence.id.clone(), ());

            Ok(Async::Ready(Some((read, write, presence, transitions, missing))))
        } else {
            Ok(Async::Ready(None))
        }
    }

    pub fn has_peer(&self, id: &PeerId) -> bool {
        self.ids.contains_key(id)
    }
}

/// Represent an emerging connection to a peer
///
/// There are two phases in the protocol, first the TCP connection should exist
/// then a Join message should tell something about the other peer. The resolved Future
/// gives the PeerCodec, the socket addr and the Join message.

pub enum Peer {
    Connecting((ConnectFuture, NetworkKey, PeerPresence, Vec<Transition>, Vec<TransitionKey>)),
    SendJoin((PeerCodecRead<TcpStream>, PeerCodecWrite<TcpStream>)),
    WaitForJoin((PeerCodecRead<TcpStream>, PeerCodecWrite<TcpStream>)),
    Ready
}

impl Peer {
    /// Initialise a full peer connection with just the address
    pub fn connect(addr: &SocketAddr, key: NetworkKey, myself: PeerPresence, tips: Vec<Transition>, missing: Vec<TransitionKey>) -> Peer {
        let addr = addr.clone();

        trace!("Connect to {:?} with {} tips", addr, tips.len());

        Peer::Connecting((TcpStream::connect(&addr), key, myself, tips, missing))
    }

    /// Initialise a full peer connection with a connected TcpStream
    pub fn send_join(socket: TcpStream, key: NetworkKey, myself: PeerPresence, tips: Vec<Transition>, missing: Vec<TransitionKey>) -> Peer {
        let addr = socket.peer_addr().unwrap();
        let (read, mut write) = new(socket, key);

        trace!("Send JOIN to {:?} with {} tips", addr, tips.len());

        write.buffer(Packet::Join(myself, tips, missing));

        Peer::SendJoin((read, write))
    }
}

/// Resolve to a fully connected peer
///
/// This future will ensure that 1. the TcpStream has been established and 2. the Join
/// message is received and valid. It is encoded as a state machine.
impl Future for Peer {
    type Item=(PeerCodecRead<TcpStream>, PeerCodecWrite<TcpStream>, PeerPresence, Vec<Transition>, Vec<TransitionKey>);
    type Error=io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut poll_again = false;
        let val = mem::replace(self, Peer::Ready);

        let new_val = match val {
            Peer::Connecting((mut socket_future, key, myself, tips, missing)) => {
                // We are here in the connecting state, the TcpStream has no connection yet. As
                // soon as the connection is established we will send the join message and then
                // poll again.
                match socket_future.poll()? {
                    Async::Ready(socket) => {poll_again = true; Peer::send_join(socket, key, myself, tips, missing)},
                    Async::NotReady => Peer::Connecting((socket_future, key, myself, tips, missing))
                }
            },

            Peer::SendJoin((read, mut write)) => {
                match write.poll_flush()? {
                    Async::Ready(_) => {poll_again = true; Peer::WaitForJoin((read, write))},
                    Async::NotReady => Peer::SendJoin((read, write))
                }
            },

            Peer::WaitForJoin((mut read, write)) => {
                // Poll the underlying socket through the PeerCodec for a Join message. If one
                // arrives, we can resolve the future.
                match read.poll()? {
                    Async::Ready(Some(Packet::Join(presence, new_transitions, missing))) => return Ok(Async::Ready((read, write, presence, new_transitions, missing))),
                    Async::Ready(None) => return {
                        error!("Got an invalid connection attempt!");
                        
                        Err(io::Error::new(io::ErrorKind::ConnectionAborted, "test"))
                    },
                    _ => Peer::WaitForJoin((read, write))
                }
            },
            _ => {
                // The finished state is unreachable, because the future won't be called again
                // after it is resolved.
                unreachable!();
            }
        };

        mem::replace(self, new_val);

        if poll_again {
            self.poll()
        } else {
            Ok(Async::NotReady)
        }
    }
}

/// Read half of the PeerCodec
///
/// The read half allows you to read arriving messages from the peer connection. It wraps a 
/// TcpStream and converts the byte stream to a message stream by implementing the Stream trait.
pub struct PeerCodecRead<T: Debug + AsyncRead> {
    read: ReadHalf<T>,
    rd: BytesMut,
    key: aead::LessSafeKey
}

/// Write half of the PeerCodec
///
/// The write half converts messages to a byte stream and send it to the peer. You can buffer
/// many messages inside the `wr` field and flush them all together with `poll_flush`.
pub struct PeerCodecWrite<T: Debug + AsyncWrite> {
    write: WriteHalf<T>,
    wr: BytesMut,
    key: aead::LessSafeKey,
    rng: rand::SystemRandom
}

/// The version field to prevent incompatible peer protocols
const VERSION: u8 = 3;

pub fn new<T: AsyncRead + AsyncWrite + Debug>(socket: T, key: NetworkKey) -> (PeerCodecRead<T>, PeerCodecWrite<T>) {
    let (read, write) = socket.split();

    // create opening/sealing keys (128bit network key)
    let ukey = aead::UnboundKey::new(&aead::AES_256_GCM, &key).unwrap();
    let read_key = aead::LessSafeKey::new(ukey);

    let ukey = aead::UnboundKey::new(&aead::AES_256_GCM, &key).unwrap();
    let write_key = aead::LessSafeKey::new(ukey);
    //let write_key = aead::SealingKey::new(key, RandomNonce { rng: rand::SystemRandom::new() });

    (
        PeerCodecRead {
            read: read,
            rd: BytesMut::new(),
            key: read_key
        },
        PeerCodecWrite {
            write: write,
            wr: BytesMut::new(),
            key: write_key,
            rng: rand::SystemRandom::new()
        }
    )
}

impl PeerCodecRead<TcpStream> {
    /// Redirect the message stream to a channel
    ///
    /// This will allow to connect many peers to a single GossipCodec. The channel unifies every
    /// arriving messages by wrapping it with the PeerId. The GossipCodec can then process the
    /// arriving messages according to the identification.
    pub fn redirect_to(self, mut sender: Sender<(PeerId, Packet)>, id: PeerId, task: Task) {
        let (task2, mut sender2, id2) = (task.clone(), sender.clone(), id.clone());
        let mut sender3 = sender.clone();

        let stream = self.map_err(|_| ())
        .and_then(move |x| {
            sender.start_send((id.clone(), x)).map_err(|err| {println!("Send error: {}", err); ()})
        })    
        .and_then(move |_| {
            task.notify();
            sender3.poll_complete().map_err(|e| eprintln!("Err = {}", e))
        })
        .for_each(move |_| Ok(()))
        .then(move |_| {
            // ugh
            sender2.try_send((id2.clone(), Packet::Close)).unwrap();
            task2.notify();

            let res: Result<()> = Ok(());
            res
        }).map_err(|_| ());


        // create a new task which handles the copying
        tokio::spawn(stream);
    }
}

impl<T: Debug + AsyncRead> PeerCodecRead<T> {
    /// Process a stream of bytes by decrypting, checking signature and unpacking the inner message
    ///
    /// The header provides version checking and data encryption in the network. It provides for
    /// this a `nonce` generated by AEAD as a unique encryption nonce and the `version` field to
    /// distinguish between different protocol versions. Finally the length field 
    ///
    /// It has the following structure:
    /// ----------------------------------------------
    /// |  96bits |  6bits  | 2bits  | 8bits..32bits |
    /// |---------|---------|--------|---------------|
    /// |  nonce  | version | additi |    length     |
    /// ---------------------------------------------|
    ///
    /// Most of the time the header has a size of 16bit for small message with size < 256bits. The
    /// `additional` field is then 0b00. For larger messages the length field can be enlarged by
    /// the bytes in the `additional` field, up to 32bit. This results in a message size < 4G.
    ///
    /// The function returns the version, the required length and then the received length.
    pub fn version_length(&self) -> Option<(u8, u32, usize)> {
        let rd = &self.rd;

        // we need at least 112bits for a header
        if rd.len() < 14 {
            return None;
        }

        // read the version (6bit) and the length of the length field (2bit)
        let (version, meta_length) = (rd[12] >> 2, rd[12] & 0b00000011);

        // now continue to check whether we can read the length field
        if rd.len() < (14 + meta_length) as usize {
            return None;
        }

        // read the length as combination of the corresponding fields
        let length = match meta_length {
            0 => (rd[13] as u32),
            1 => (rd[13] as u32) | (rd[14] as u32) << 8,
            2 => (rd[13] as u32) | (rd[14] as u32) << 8 | (rd[15] as u32) << 16,
            3 => (rd[13] as u32) | (rd[14] as u32) << 8 | (rd[15] as u32) << 16 | (rd[16] as u32) << 24,
            _ => unreachable!()
        };

        Some((version, length, self.rd.len() - meta_length as usize - 14))
    }

    pub fn parse_packet(&mut self) -> Result<Packet> {
        // read the header
        let (version, required_length, buffer_length) = match self.version_length() {
            Some((a,b,c)) => (a,b,c),
            None => return Err(Error::NotEnoughBytes)
        };

        // check the version
        if version != VERSION {
            trace!("Parse packet with invalid version {} != {}", version, VERSION);
            return Err(Error::WrongVersion);
        }

        //println!("Requied length: {}", required_length);

        // continue till we have enough bytes
        if required_length as usize > buffer_length {
            return Err(Error::NotEnoughBytes);
        }

        // if we have reached the required byte number, read in the buffer
        let meta_length = (self.rd[12] & 0b00000011) as usize;
        let mut buf = self.rd.split_to(required_length as usize + 14 + meta_length);

        // get once and create a copy
        let nonce = Nonce::try_assume_unique_for_key(&buf[0..12]).unwrap();

        //println!("Nonce {:?}", nonce);
        //println!("Read buf {:?}", buf.len());

        // decrypt and check signature
        self.key.open_within(
            nonce,
            Aad::empty(),
            &mut buf,
            (14+meta_length)..
        ).map_err(|_| {
            error!("Cryptographic failure, probably connection attempt with wrong network key!");

            Error::Cryptography
        })?;

        // now try to deserialise it to a message, we have to skip the header bytes
        deserialize::<Packet>(&buf).map_err(|_| Error::Deserialize)
    }

    /// Try to read in some data from the byte stream
    fn fill_read_buf(&mut self) -> Poll<(), io::Error> {
        loop {
            self.rd.reserve(8192*2);
            let read = self.read.read_buf(&mut self.rd);

            let n = match read {
                Ok(Async::Ready(n)) => {
                    trace!("Read {} bytes into buffer", n);
                    n
                },
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(err) => {
                    if err.kind() == ErrorKind::WouldBlock {
                        return Ok(Async::NotReady);
                    } else {
                        return Err(err);
                    }
                }
            };

            if n == 0 {
                return Ok(Async::Ready(()));
            }
        }
    }
}

impl<T: Debug + AsyncWrite> PeerCodecWrite<T> {
    /// Buffer a message to the byte stream
    ///
    /// First we serialise the message to a byte representation, and then
    /// calculates the metadata values. After this we can push the block to the
    /// data stream.
    pub fn buffer(&mut self, message: Packet) {
        //println!("Buffer: {:?}", message);
        if let Ok(mut buf) = serialize(&message) {
            trace!("Buffer {} bytes", buf.len());
            // encrypt and sign our data with the network key
            // TODO generate nonce
            let mut nonce_buf = [0u8; 12];

            self.rng.fill(&mut nonce_buf).unwrap();

            let nonce = Nonce::assume_unique_for_key(nonce_buf);

            // enlarge buffer for additional 128bits
            //let tag_len = self.key.algorithm().tag_len() ;

            //buf.append(&mut vec![0u8; tag_len]);

            self.key.seal_in_place_append_tag(
                nonce, // a unique and random key created for each message
                Aad::empty(),
                &mut buf, // buffer which will be overwritten with the encrypted and signed message
            ).unwrap();

            let buf_len = buf.len();

            // calculate the value of the `additional` field by couting the zeros of the buffer
            // length
            let length = (32 - (buf_len as u32).leading_zeros()) as u8 / 8;


            // we can't transmit more than 4G at once, should never happen anyway
            if length > 4 {
                return;
            }

            // check if remaining space is sufficient
            let rem = self.wr.capacity() - self.wr.len();

            if rem < length as usize + 14 + buf_len {
                let new_size = self.wr.len() + rem + length as usize + 14 + buf_len;
                self.wr.reserve(new_size);
            }

            // write the nonce 
            self.wr.put(&nonce_buf[..]);

            // put the `version` and `additional` field to the write buffer
            self.wr.put_u8(VERSION << 2 | length);

            // write the buffer length
            let mut buf_length = buf_len;
            for _ in 0..length+1 {
                self.wr.put_u8((buf_length & 0xFF) as u8);

                buf_length = buf_length >> 8;
            }

            // put the message itself to the buffer
            self.wr.put(&buf[0..buf_len]);
        }
    }

        /// Flush the whole write buffer to the underlying socket
    pub fn poll_flush(&mut self) -> Poll<(), io::Error> {
        while !self.wr.is_empty() {
            let n = try_ready!(self.write.poll_write(&self.wr));

            trace!("Flushed {} left {}", n, self.wr.len());

            assert!(n > 0);

            self.wr.split_to(n);
        }

        self.write.poll_flush()
    }

    pub fn shutdown(mut self) -> Poll<(), io::Error> {
        self.write.shutdown()
    }

    pub fn is_empty(&self) -> bool {
        self.wr.len() == 0
    }
}

/// Packet stream consuming the underlying byte stream. bytes_stream -> message_stream
///
/// The PeerCodec consumes a byte stream and tries to construct messages from it. The messages
/// can then be used by the GossipCodec to communicate with a peer. 
impl<T: Debug + AsyncRead> Stream for PeerCodecRead<T> {
    type Item = Packet;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // read new data that might have been received off the socket
        // track if the socket is closed here
        let res = self.fill_read_buf()
            .map_err(|x| {println!("{:?}", x); x})?;
            
        let is_closed = res.is_ready();

        if is_closed {
            // the socket seems to have closed after the last call, signal that
            // the stream is finished, because we can't receive any new data
            return Ok(Async::Ready(None));
        }
        
        let res = self.parse_packet();

        match res {
            Ok(msg) => Ok(Async::Ready(Some(msg))),
            // peer has a wrong version, close connection
            Err(Error::WrongVersion) => Ok(Async::Ready(None)),
            // peer sent probably a wrong key, close connection
            Err(Error::Cryptography) => Ok(Async::Ready(None)),
            // in all other cases we await more bytes
            Err(Error::Deserialize) => Ok(Async::NotReady),
            Err(Error::NotEnoughBytes) => Ok(Async::NotReady),
            _ => Ok(Async::NotReady)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{new, Packet, Transition, PeerId};
    use std::io::Cursor;
    use bytes::BufMut;
    use ring::rand::{SecureRandom, SystemRandom};
    use test::Bencher;

    #[test]
    fn read_write() {
        let mut buf = Cursor::new(Vec::new());
        let rng = SystemRandom::new();

        let mut tmp = vec![0u8; 65536];
        rng.fill(&mut tmp).unwrap();

        let packet = Packet::Push(Transition::new(Vec::new(), Vec::new(), tmp));

        let mut key = [0u8; 32];
        rng.fill(&mut key).unwrap();

        let (mut read, mut write) = new(&mut buf, key);

        write.buffer(packet.clone());

        read.rd.reserve(write.wr.len());
        read.rd.put_slice(&write.wr.as_ref());

        assert_eq!(read.parse_packet().unwrap(), packet);
    }

    // size of a random payload
    const BUF_SIZE: usize = 8192;

    #[bench]
    fn bench_encrypt(b: &mut Bencher) {
        let mut buf = Cursor::new(Vec::new());
        let rng = SystemRandom::new();

        let mut tmp = vec![0u8; BUF_SIZE];
        rng.fill(&mut tmp).unwrap();

        let packet = Packet::Push(Transition::new(Vec::new(), Vec::new(), tmp));

        let mut key = [0u8; 32];
        rng.fill(&mut key).unwrap();

        let (_, mut write) = new(&mut buf, key);

        b.iter(|| write.buffer(packet.clone()));
    }

    #[bench]
    fn bench_decrypt(b: &mut Bencher) {
        let mut buf = Cursor::new(Vec::new());
        let rng = SystemRandom::new();

        let mut tmp = vec![0u8; BUF_SIZE];
        rng.fill(&mut tmp).unwrap();

        let packet = Packet::Push(Transition::new(Vec::new(), Vec::new(), tmp));

        let mut key = [0u8; 32];
        rng.fill(&mut key).unwrap();

        let (mut read, mut write) = new(&mut buf, key);

        write.buffer(packet.clone());

        read.rd.reserve(write.wr.len());

        b.iter(|| {
            read.rd.reserve(write.wr.len());
            read.rd.put_slice(&write.wr.as_ref());

            read.parse_packet().unwrap();
        });

    }
}
