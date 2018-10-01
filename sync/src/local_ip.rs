use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::process::Command;

pub fn get() -> Result<Vec<IpAddr>, io::Error> {
    let output = Command::new("hostname").args(&["-i"]).output()?;
    let stdout = String::from_utf8(output.stdout).unwrap();

    let res = stdout.trim().split(" ")
        .filter_map(|x| 
             x.parse::<Ipv4Addr>().map(|x| IpAddr::from(x))
             .or_else(|_| x.parse::<Ipv6Addr>().map(|x| IpAddr::from(x)))
             .ok()
        )
        .collect::<Vec<IpAddr>>();

    Ok(res)
}
