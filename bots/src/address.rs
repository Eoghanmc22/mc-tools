use std::io::{self, ErrorKind};
use std::net::{SocketAddr, ToSocketAddrs};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct MinecraftAddress(pub SocketAddr);

impl FromStr for MinecraftAddress {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.to_socket_addrs()
            .or_else(|_| (s, 25565).to_socket_addrs())?
            .into_iter()
            .filter(|it| it.is_ipv4())
            .map(MinecraftAddress)
            .next()
            .ok_or_else(|| ErrorKind::NotFound.into())
    }
}
