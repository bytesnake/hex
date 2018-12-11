//! Discover other peers in the same network with UDP broadcast.
//!
//! This module uses a very simple UDP broadcast and reply approach to find a contact addr to a
//! peer-to-peer network. This address can then be used to bootstrap the join process.
//! The probing packet consists of [version, 0x00, 0x00, 0x01], allowing the
//! replying server to ignore in incompatible peers

use std::io::{self, Write, ErrorKind};
use std::time::Instant;
use std::net::UdpSocket as UdpSocket2;

use tokio::net::UdpSocket;
use futures::{Async, Stream};
use std::net::{SocketAddrV4, Ipv4Addr, SocketAddr, IpAddr};
use std::os::unix::io::AsRawFd;

use nix::sys::socket::{self, sockopt::ReusePort};

use local_ip;

struct Packet {
    version: u8,
    key: [u8; 16],
    contact: SocketAddr
}

/// Reply to probing packets with the correct version field
pub struct Discover {
    socket: UdpSocket,
    buf: Vec<u8>,
    answer_to: Option<(usize, SocketAddr)>,
    version: u8
}

impl Stream for Discover {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        loop {
            if let Some((nbuf, addr)) = self.answer_to {
                if nbuf == 4 && self.buf[0..4] == [self.version, 0x00, 0x00, 0x01] {
                    try_ready!(self.socket.poll_send_to(&self.buf[..nbuf], &addr));
                }

                self.answer_to = None
            }

            self.answer_to = Some(try_ready!(self.socket.poll_recv_from(&mut self.buf)));
        }
    }
}

impl Discover {
    /// Create a new reply server, only replying to the specified version
    pub fn new(version: u8) -> Discover {
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8004);
        let socket = UdpSocket::bind(&addr.into()).unwrap();
        socket::setsockopt(socket.as_raw_fd(), ReusePort, &true).unwrap();
        socket.set_broadcast(true).unwrap();

        Discover {
            buf: vec![0; 16],
            answer_to: None,
            version,
            socket
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
    version: u8,
    buf: Vec<u8>,
    local_addrs: Vec<IpAddr>,
}

impl Beacon {
    /// Create a new `Beacon` struct which tries to discover peers at an interval `interval` and
    /// with version `version`
    pub fn new(version: u8) -> Beacon {
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8004);
        let socket = UdpSocket2::bind(&addr).unwrap();
        socket::setsockopt(socket.as_raw_fd(), ReusePort, &true).unwrap();
        socket.set_broadcast(true).unwrap();
        socket.set_nonblocking(true).unwrap();

        Beacon {
            buf: vec![0; 16],
            local_addrs: local_ip::get().unwrap(),
            version,
            socket
        }
    }

    pub fn wait(&mut self, nsecs: u64) -> Option<SocketAddr> {
        let start = Instant::now();
        let mut last_sent = Instant::now();
        print!("Search for peers ");
        std::io::stdout().flush().unwrap();

        loop {
            if Instant::now().duration_since(start).as_secs() >= nsecs {
                println!(" nobody found!");
                return None;
            }

            if Instant::now().duration_since(last_sent).as_millis() > 500 {
                print!(".");
                std::io::stdout().flush().unwrap();

                self.socket.send_to(
                        &[self.version, 0x00, 0x00, 0x01], 
                        &(SocketAddrV4::new(Ipv4Addr::BROADCAST, 8004))).unwrap();

                last_sent = Instant::now();
            }

            'inner: loop {
                let (nread, addr) = match self.socket.recv_from(&mut self.buf) {
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => break 'inner,
                    Ok(a) => a,
                    _ => return None
                };

                //println!("{}", addr);
                if !self.local_addrs.contains(&addr.ip()) {
                    if nread == 4 && self.buf[0..4] == [self.version, 0x00, 0x00, 0x01] {
                        println!(" found peer at {}", addr);
                        return Some(addr);
                    }
                }
            }

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
