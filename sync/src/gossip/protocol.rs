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
use tokio::net::{TcpStream, ConnectFuture};
use bytes::{BytesMut, BufMut};
use bincode::{deserialize, serialize};

use gossip::{PeerId, PeerPresence};

/// Peer-to-Peer message
/// 
/// The protocol is not very complex. After establishing a connection
/// every peer should send a Join message as a handshake. The peers
/// can then be requested with the GetPeers message. A push sends
/// a new block of data to a Peer.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Packet {
    Join(PeerPresence),
    GetPeers(Option<Vec<PeerPresence>>),
    Push(Vec<u8>),
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

    pub fn poll(&mut self) -> Result<Async<Option<(PeerCodecRead<TcpStream>, PeerCodecWrite<TcpStream>, PeerPresence)>>, io::Error> {
        if let Some((read, write, presence)) = try_ready!(self.awaiting.poll()) {
            self.ids.insert(presence.id.clone(), ());

            Ok(Async::Ready(Some((read, write, presence))))
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
    Connecting((ConnectFuture, PeerPresence)),
    WaitForJoin((PeerCodecRead<TcpStream>, PeerCodecWrite<TcpStream>)),
    Ready
}

impl Peer {
    /// Initialise a full peer connection with just the address
    pub fn connect(addr: &SocketAddr, myself: PeerPresence) -> Peer {
        Peer::Connecting((TcpStream::connect(addr), myself))
    }

    /// Initialise a full peer connection with a connected TcpStream
    pub fn wait_for_join(socket: TcpStream, myself: PeerPresence) -> Peer {
        //println!("Send join from {}", myself.id);
        let (read, mut write) = new(socket);

        write.buffer(Packet::Join(myself));
        write.poll_flush().unwrap();

        Peer::WaitForJoin((read, write))
    }
}

/// Resolve to a fully connected peer
///
/// This future will ensure that 1. the TcpStream has been established and 2. the Join
/// message is received and valid. It is encoded as a state machine.
impl Future for Peer {
    type Item=(PeerCodecRead<TcpStream>, PeerCodecWrite<TcpStream>, PeerPresence);
    type Error=io::Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        let mut poll_again = false;
        let val = mem::replace(self, Peer::Ready);

        let new_val = match val {
            Peer::Connecting((mut socket_future, myself)) => {
                // We are here in the connecting state, the TcpStream has no connection yet. As
                // soon as the connection is established we will send the join message and then
                // poll again.
                match socket_future.poll() {
                    Ok(Async::Ready(socket)) => {poll_again = true; Peer::wait_for_join(socket, myself)},
                    Err(err) => return Err(err),
                    _ => Peer::Connecting((socket_future, myself))
                }
            },

            Peer::WaitForJoin((mut read, write)) => {
                // Poll the underlying socket through the PeerCodec for a Join message. If one
                // arrives, we can resolve the future.
                match read.poll() {
                    Ok(Async::Ready(Some(Packet::Join(presence)))) => return Ok(Async::Ready((read, write, presence))),
                    Err(err) => return Err(err),
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
}

/// Write half of the PeerCodec
///
/// The write half converts messages to a byte stream and send it to the peer. You can buffer
/// many messages inside the `wr` field and flush them all together with `poll_flush`.
pub struct PeerCodecWrite<T: Debug + AsyncWrite> {
    write: WriteHalf<T>,
    wr: BytesMut
}

/// The version field to prevent incompatible peer protocols
const VERSION: u8 = 1;

pub fn new(socket: TcpStream) -> (PeerCodecRead<TcpStream>, PeerCodecWrite<TcpStream>) {
    let (read, write) = socket.split();

    (
        PeerCodecRead {
            read: read,
            rd: BytesMut::new()
        },
        PeerCodecWrite {
            write: write,
            wr: BytesMut::new()
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
            task.notify();
            sender.start_send((id.clone(), x)).map_err(|err| {println!("Send error: {}", err); ()})
        })    
        .and_then(move |_| sender3.poll_complete().map_err(|_| ()))
        .for_each(move |_| {
            //sender.send((id.clone(), x)).wait();

            Ok(())
        })
        .then(move |_| {
            // ugh
            sender2.try_send((id2.clone(), Packet::Close)).unwrap();
            task2.notify();

            let res: Result<(), ()> = Ok(());
            res
        }).map_err(|_| ());


        // create a new task which handles the copying
        tokio::spawn(stream);
    }
}

impl<T: Debug + AsyncRead> PeerCodecRead<T> {
    /// Parse the metadata of a byte stream
    ///
    /// The header has the task to describe the protocol version and the length of the data field.
    /// The protocol version prevents mixing different protocol together and the
    /// length field allows a proper read-in of the byte stream.
    ///
    /// It has the following structure:
    /// -------------------------------
    /// |  6bits  | 2bits | 8bits..32bits |
    /// |---------|-------|---------------|
    /// |version  |additi |    length     |
    /// ----------------------------------
    ///
    /// Most of the time the header has a size of 16bit for small message with size < 256bits. The
    /// `additional` field is then 0b00. For larger messages the length field can be enlarged by
    /// the bytes in the `additional` field, up to 32bit. This results in a message size < 4G.
    ///
    /// The function returns the version, the required length and then the received length.
    pub fn version_length(&self) -> Option<(u8, u32, usize)> {
        let rd = &self.rd;

        // check first if we can read the metadata
        if rd.len() < 2 {
            return None;
        }

        // read the version (6bit) and the length of the length field (2bit)
        let (version, meta_length) = (rd[0] >> 2, rd[0] & 0b00000011);

        // now continue to check whether we can read the length field
        if rd.len() < (2 + meta_length + 1) as usize {
            return None;
        }

        // read the length as combination of the corresponding fields
        let length = match meta_length {
            0 => (rd[1] as u32),
            1 => (rd[1] as u32) | (rd[2] as u32) << 8,
            2 => (rd[1] as u32) | (rd[2] as u32) << 8 | (rd[3] as u32) << 16,
            3 => (rd[1] as u32) | (rd[2] as u32) << 8 | (rd[3] as u32) << 16 | (rd[4] as u32) << 24,
            _ => unreachable!()
        };

        Some((version, length, self.rd.len() - 1 - meta_length as usize - 1))
    }

    /// Try to read in some data from the byte stream
    fn fill_read_buf(&mut self) -> Result<Async<()>, io::Error> {
        loop {
            self.rd.reserve(1024);
            let read = self.read.read_buf(&mut self.rd);

            let n = match read {
                Ok(Async::Ready(n)) => n,
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
        if let Ok(buf) = serialize(&message) {
            // calculate the value of the `additional` field by couting the zeros of the buffer
            // length
            let length = (32 - (buf.len() as u32).leading_zeros()) as u8 / 8;

            // we can't transmit more than 4G at once, should never happen anway
            if length > 4 {
                return;
            }

            // check if remaining space is sufficient
            let rem = self.wr.capacity() - self.wr.len();
            if rem < length as usize + 2 + buf.len() {
                let new_size = self.wr.len() + rem + length as usize + 2+ buf.len();
                self.wr.reserve(new_size);
            }


            // put the `version` and `additional` field to the write buffer
            self.wr.put_u8(VERSION << 2 | length);

            // write the buffer length
            let mut buf_length = buf.len();
            for _ in 0..length+1 {
                self.wr.put_u8((buf_length & 0xFF) as u8);

                buf_length = buf_length >> 8;
            }

            // put the message itself to the buffer
            self.wr.put(buf);
        }
    }

    /// Flush the whole write buffer to the underlying socket
    pub fn poll_flush(&mut self) -> Poll<(), io::Error> {
        while !self.wr.is_empty() {
            'inner: loop {
                match self.write.poll_write(&self.wr) {
                    Ok(Async::Ready(n)) => {
                        assert!(n > 0);

                        let _ = self.wr.split_to(n);

                        break 'inner;
                    },
                    Ok(Async::NotReady) => {},
                    Err(err) => {
                        return Err(err);
                    }
                }
            }
        }

        self.write.poll_flush().unwrap();

        Ok(Async::Ready(()))
    }

    pub fn shutdown(self) {
        io::shutdown(self.write);
    }
}

/// Packet stream consuming the underlying byte stream. bytes_stream -> message_stream
///
/// The PeerCodec consumes a byte stream and tries to construct messages from it. The messages
/// can then be used by the GossipCodec to communicate with a peer. 
impl<T: Debug + AsyncRead> Stream for PeerCodecRead<T> {
    type Item = Packet;
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        // read new data that might have been received off the socket
        // track if the socket is closed here
        let is_closed = self.fill_read_buf().map_err(|x| {println!("{:?}", x); x})?.is_ready();

        if is_closed {
            // the socket seems to have closed after the last call, signal that
            // the stream is finished, because we can't receive any new data
            //return Ok(Async::Ready(None));
            return Ok(Async::Ready(None));
        }

        // read the header
        let (version, required_length, buffer_length) = match self.version_length() {
            Some((a,b,c)) => (a,b,c),
            None => return Ok(Async::NotReady)
        };

        // continue till we have enough bytes
        if version != VERSION || (required_length as usize) > buffer_length {
            return Ok(Async::NotReady);
        }

        // if we have reached the required byte number, read in the buffer
        let meta_length = (self.rd[0] & 0b00000011) as usize;
        let buf = self.rd.split_to(required_length as usize + 2 + meta_length);

        // now try to deserialise it to a message, we have to skip the header bytes
        let message = deserialize::<Packet>(&buf[2+meta_length..]);

        if let Ok(message) = message {
            //println!("Gossip: got {:?}", message);
            //println!("");
            // we have done all the required steps, forward the new message
            return Ok(Async::Ready(Some(message)));
        } else {
            // the codec needs more data before it can construct the message sucessfully
            Ok(Async::NotReady)
        }
    }
}
