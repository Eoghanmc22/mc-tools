use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct MinecraftAddress(pub SocketAddr);

impl FromStr for MinecraftAddress {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.to_socket_addrs()
            .or_else(|_| (s, 25565).to_socket_addrs())
            .map(|mut iter| iter.next().unwrap())
            .map(MinecraftAddress)
    }
}
