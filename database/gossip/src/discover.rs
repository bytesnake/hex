//! Discover other peers in the same network with UDP broadcast.
//!
//! This module uses a very simple UDP broadcast and reply approach to find a contact addr to a
//! peer-to-peer network. This address can then be used to bootstrap the join process.
//! The probing packet consists of [version, 0x00, 0x00, 0x01], allowing the
//! replying server to ignore in incompatible peers

use std::io::{self, Write, ErrorKind};
use std::time::{Instant, Duration};
use std::thread;
use std::net::UdpSocket as UdpSocket2;

use nix::ifaddrs::getifaddrs;
use nix::sys::socket::SockAddr;
use tokio::{net::UdpSocket, reactor::Handle};
use futures::{Async, Stream};
use std::net::{SocketAddrV4, Ipv4Addr, SocketAddr, IpAddr};

use net2::UdpBuilder;
use bincode::{serialize, deserialize};
use ring::digest;

use local_ip;
use protocol::NetworkKey;

#[derive(Deserialize, Serialize, Debug)]
struct Packet {
    version: u8,
    key: [u8; 32],
    contact_port: u16
}

impl Packet {
    pub fn new(version: u8, network: NetworkKey, contact_port: u16) -> Packet {
        let mut key = [0u8; 32];

        // hash the network key
        let hash = digest::digest(&digest::SHA256, &network.as_ref());
        key.copy_from_slice(&hash.as_ref());

        Packet {
            version, contact_port, key
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        serialize(self).unwrap()
    }

    pub fn from_vec(buf: &[u8]) -> Option<Packet> {
        deserialize(buf).ok()
    }
}

/// Reply to probing packets with the correct version field
pub struct Discover {
    socket: UdpSocket,
    buf: Vec<u8>,
    answer_to: Option<(usize, SocketAddr)>,
    packet: Packet,
    ips: Vec<IpAddr>
}

impl Stream for Discover {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        loop {
            if let Some((nbuf, addr)) = self.answer_to {
                if let Some(packet) = Packet::from_vec(&self.buf[0..nbuf]) {
                    if packet.key == self.packet.key && 
                       packet.version == self.packet.version &&
                       (packet.contact_port != self.packet.contact_port ||
                        !self.ips.contains(&addr.ip())) {

                        let buf = self.packet.to_vec();

                        try_ready!(self.socket.poll_send_to(&buf, &addr));

                        self.answer_to = None;
                    }
                }
            }


            //println!("{:?}", self.answer_to);
            //println!("{:?}", try_ready!(self.socket.poll_recv_from(&mut self.buf)));
            self.answer_to = Some(try_ready!(self.socket.poll_recv_from(&mut self.buf)));
        }
    }
}

impl Discover {
    /// Create a new reply server, only replying to the specified version
    pub fn new(version: u8, network: NetworkKey, contact_port: u16) -> Discover {
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8004);
        let socket = UdpBuilder::new_v4().unwrap()
            .reuse_address(true).unwrap()
            //.reuse_port(true).unwrap()
            .bind(addr).unwrap();

        socket.set_broadcast(true).unwrap();
        //socket.set_nonblocking(true).unwrap();

        let packet = Packet::new(version, network, contact_port);

        Discover {
            buf: vec![0; 1024],
            answer_to: None,
            packet,
            socket: UdpSocket::from_std(socket, &Handle::default()).unwrap(),
            ips: local_ip::get().unwrap()
        }
    }
}

/// Probe into an unknown network structure and discover other peers. 
///
/// If no peer replies after two seconds, the `Future` will be resolved with `Option::None`
///
/// ## Example
/// ```rust
/// extern crate futures;
/// extern crate hex_gossip;
///
/// use futures::future::Future;
/// use hex_gossip::discover::Beacon;
///
/// let beacon = Beacon::new(0, 200)
///     .map(|addr| println!("Discovered contact at {:?}", addr))
///     .map_err(|err| eprintln!("Err = {:?}", err));
///
/// tokio::run(beacon);
/// ```
pub struct Beacon {
    socket: UdpSocket2,
    buf: Vec<u8>,
    packet: Packet
}

impl Beacon {
    /// Create a new `Beacon` struct which tries to discover peers at an interval `interval` and
    /// with version `version`
    pub fn new(version: u8, network: NetworkKey, contact_port: u16) -> Beacon {
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8004);
        
        let socket = UdpBuilder::new_v4().unwrap()
            .reuse_address(true).unwrap()
            //.reuse_port(true).unwrap()
            .bind(addr).unwrap();

        socket.set_broadcast(true).unwrap();
        socket.set_nonblocking(true).unwrap();

        Beacon {
            buf: vec![0; 1024],
            socket,
            packet: Packet::new(version, network, contact_port)
        }
    }

    pub fn wait(&mut self, nsecs: u64) -> Option<SocketAddr> {
        let start = Instant::now();
        let mut last_sent = Instant::now();
        print!("Search for peers ");
        std::io::stdout().flush().unwrap();

        let my_ips: Vec<IpAddr> = getifaddrs().unwrap().filter_map(|x| {
            match x.address{
                Some(SockAddr::Inet(inet)) => Some(inet.to_std().ip()),
                _ => None
            }
        }).collect();

        loop {
            if Instant::now().duration_since(start).as_secs() >= nsecs {
                println!(" nobody found!");
                return None;
            }

            if Instant::now().duration_since(last_sent).as_millis() > 500 {
                print!(".");
                std::io::stdout().flush().unwrap();

                if let Err(err) = self.socket.send_to(
                        &self.packet.to_vec(), 
                        &(SocketAddrV4::new(Ipv4Addr::BROADCAST, 8004))) {
                    eprintln!(" could not send ping {}", err);

                    return None;
                }

                last_sent = Instant::now();
            }

            'inner: loop {
                let (nread, mut addr) = match self.socket.recv_from(&mut self.buf) {
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        break 'inner
                    },
                    Ok(a) => a,
                    _ => return None
                };

                // check if request originates from our address and the corresponding port
                if let Some(packet) = Packet::from_vec(&self.buf[0..nread]) {
                    if (!my_ips.contains(&addr.ip()) || packet.contact_port != self.packet.contact_port) &&
                        packet.key == self.packet.key && 
                        packet.version == self.packet.version {
                            addr.set_port(packet.contact_port);

                            println!(" found peer at {}", addr);
                            return Some(addr);
                    }
                }
            }

            thread::sleep(Duration::from_millis(50));

        }
    }
}

/*#[cfg(test)]
mod tests {
    use super::{Beacon, Discover};
    use futures::{Future, Stream};
    use tokio;

    #[test]
    fn send_beacon() {
        let beacon = Beacon::new(1, 500);

        tokio::run(beacon.map_err(|e| println!("Beacon error = {:?}", e)).map(|x| println!("Beacon got = {:?}", x)));
    }

    #[test]
    fn discover() {
        let discover = Discover::new(1);

        tokio::run(discover
           .for_each(|x| { println!("Detected peer = {:?}", x); Ok(())})
           .map_err(|_| ())
        );
    }
}*/
