use std::io;
use std::time::{Instant, Duration};

use tokio::net::UdpSocket;
use tokio::timer::Interval;
use futures::{Async, Future, Stream};
use std::net::{SocketAddrV4, Ipv4Addr, SocketAddr, IpAddr};

use local_ip;

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
    pub fn new(version: u8) -> Discover {
        Discover {
            socket: UdpSocket::bind(&("0.0.0.0:8004".parse().unwrap())).unwrap(),
            buf: vec![0; 16],
            answer_to: None,
            version
        }
    }
}

pub struct Beacon {
    socket: UdpSocket,
    version: u8,
    interval: Interval,
    buf: Vec<u8>,
    local_addrs: Vec<IpAddr>
}

impl Future for Beacon {
    type Item=SocketAddr;
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        match self.interval.poll() {
            Ok(Async::Ready(_)) => { 
                println!("POLL"); 

                try_ready!(self.socket.poll_send_to(
                        &[self.version, 0x00, 0x00, 0x01], 
                        &(SocketAddrV4::new(Ipv4Addr::BROADCAST, 8004).into())));
            }
            _ => {}
        }

        let (nread, addr) = try_ready!(self.socket.poll_recv_from(&mut self.buf));

        if nread == 4 && self.buf[0..4] == [self.version, 0x00, 0x00, 0x01] {
            if self.local_addrs.contains(&addr.ip()) {
                Ok(Async::NotReady)
            } else {
                Ok(Async::Ready(addr))
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}

impl Beacon {
    pub fn new(version: u8, interval: u64) -> Beacon {
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8004);
        let socket = UdpSocket::bind(&addr.into()).unwrap();
        socket.set_broadcast(true).unwrap();

        Beacon {
            interval: Interval::new(Instant::now(), Duration::from_millis(interval)),
            buf: vec![0; 16],
            local_addrs: local_ip::get().unwrap(),
            version,
            socket
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Beacon;
    use futures::Future;
    use tokio;

    #[test]
    fn send_beacon() {
        let beacon = Beacon::new(1, 500);

        tokio::run(beacon.map_err(|e| println!("Beacon error = {:?}", e)).map(|x| println!("Beacon got = {:?}", x)));
    }
}
