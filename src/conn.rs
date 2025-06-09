use std::{future::Future, io::{ErrorKind, Result}, net::SocketAddr, pin::Pin};
use async_trait::async_trait;
use futures::future::pending;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use tokio::{net::{TcpListener, TcpStream, UdpSocket}};

use crate::{config, player::Player};

#[derive(FromPrimitive, ToPrimitive)]
pub enum UdpPacketMode {
    Pos,
}

#[derive(FromPrimitive, ToPrimitive)]
pub enum TcpPacketMode {
    Seeds,
    Place,
    Hold,
}

#[async_trait]
pub trait ConnTrait {
    async fn send_seeds(&self, p1_seed: u64, p2_seed: u64) -> Result<()>;
    async fn send_place(&self, player: &Player) -> Result<()>;
    async fn send_hold(&self) -> Result<()>;
    async fn send_pos(&self, player: &Player) -> Result<()>;
    fn recv_udp<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(UdpPacketMode, [u8; 63])>> + Send + 'a>>;
    fn recv_tcp<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(TcpPacketMode, [u8; 63])>> + Send + 'a>>;
    fn is_multiplayer(&self) -> bool;
    fn is_host(&self) -> bool;
}

pub struct Conn {
    pub tcp_stream: TcpStream,
    pub udp_socket: UdpSocket,
    pub is_host: bool,
}

impl Conn {
    async fn tcp_listen(tcp_listener: &TcpListener) -> (TcpStream, SocketAddr) {
        loop {
            if let Ok(socket) = tcp_listener.accept().await {
                return socket
            }
        }
    }

    pub async fn establish_connection(is_host: bool) -> Result<Self> {
        if is_host {
            let tcp_listener = TcpListener::bind(*config::BIND_ADDR).await?;
            let (tcp_stream, peer_addr) = Conn::tcp_listen(&tcp_listener).await;

            let udp_socket = UdpSocket::bind(*config::BIND_ADDR).await?;
            udp_socket.connect(peer_addr).await?;

            Ok(Conn { udp_socket, tcp_stream, is_host })
        } else {
            let tcp_stream = TcpStream::connect(*config::CONN_ADDR).await?;

            let udp_socket = UdpSocket::bind(tcp_stream.local_addr().unwrap()).await?;
            udp_socket.connect(*config::CONN_ADDR).await?;

            Ok(Conn { udp_socket, tcp_stream, is_host })
        }
    }

    async fn send_tcp(&self, mode: TcpPacketMode, payload: &[u8]) -> Result<()> {
        loop {
            self.tcp_stream.writable().await?;
            let mut buf = Vec::new();
            buf.extend_from_slice(&mode.to_u8().unwrap().to_le_bytes());
            buf.extend_from_slice(payload);
            match self.tcp_stream.try_write(&mut buf) {
                Ok(_) => {
                    return Ok(());
                },
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    continue;
                },
                Err(e) => {
                    return Err(e.into());
                },
            }
        }
    }

    async fn send_udp(&self, mode: UdpPacketMode, payload: &[u8]) -> Result<()> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&mode.to_u8().unwrap().to_le_bytes());
        buf.extend_from_slice(payload);
        self.udp_socket.send(&mut buf).await?;
        Ok(())
    }
}

#[async_trait]
impl ConnTrait for Conn {
    async fn send_seeds(&self, p1_seed: u64, p2_seed: u64) -> Result<()> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&p1_seed.to_le_bytes());
        buf.extend_from_slice(&p2_seed.to_le_bytes());
        self.send_tcp(
            TcpPacketMode::Seeds,
            &buf,
        ).await?;
        Ok(())
    }

    async fn send_place(&self, player: &Player) -> Result<()> {
        self.send_tcp(
            TcpPacketMode::Place,
            &player.falling.geometry.to_bytes(),
        ).await?;
        Ok(())
    }

    async fn send_hold(&self) -> Result<()> {
        self.send_tcp(
            TcpPacketMode::Hold,
            b"",
        ).await?;
        Ok(())
    }

    async fn send_pos(&self, player: &Player) -> Result<()> {
        self.send_udp(
            UdpPacketMode::Pos,
            &player.falling.geometry.to_bytes(),
        ).await?;
        Ok(())
    }

    fn recv_udp<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(UdpPacketMode, [u8; 63])>> + Send + 'a>> {
        Box::pin(async move {
            let mut buf = [0u8; 64];
            self.udp_socket.recv(&mut buf).await?;
            let mode_bytes: &[u8; 1] = buf[0..1].try_into().unwrap();
            let mode = UdpPacketMode::from_u8(u8::from_le_bytes(*mode_bytes)).unwrap();
            let payload: [u8; 63] = buf[1..64].try_into().unwrap();
            Ok((mode, payload))
        })
    }

    fn recv_tcp<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(TcpPacketMode, [u8; 63])>> + Send + 'a>> {
        Box::pin(async move {
            let mut buf = [0u8; 64];
            loop {
                self.tcp_stream.readable().await?;
                match self.tcp_stream.try_read(&mut buf) {
                    Ok(_) => {
                        let mode_bytes: &[u8; 1] = buf[0..1].try_into().unwrap();
                        let mode = TcpPacketMode::from_u8(u8::from_le_bytes(*mode_bytes)).unwrap();
                        let payload: [u8; 63] = buf[1..64].try_into().unwrap();
                        return Ok((mode, payload));
                    },
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        continue;
                    },
                    Err(e) => {
                        return Err(e.into());
                    },
                }
            }
        })
    }

    fn is_multiplayer(&self) -> bool {
        true
    }

    fn is_host(&self) -> bool {
        self.is_host
    }
}

pub struct DummyConn;

#[async_trait]
impl ConnTrait for DummyConn {
    async fn send_seeds(&self, _p1_seed: u64, _p2_seed: u64) -> Result<()> {
        Ok(())
    }

    async fn send_place(&self, _player: &Player) -> Result<()> {
        Ok(())
    }

    async fn send_hold(&self) -> Result<()> {
        Ok(())
    }

    async fn send_pos(&self, _player: &Player) -> Result<()> {
        Ok(())
    }

    fn recv_udp<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(UdpPacketMode, [u8; 63])>> + Send + 'a>> {
        Box::pin(pending())
    }

    fn recv_tcp<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(TcpPacketMode, [u8; 63])>> + Send + 'a>> {
        Box::pin(pending())
    }

    fn is_multiplayer(&self) -> bool {
        false
    }

    fn is_host(&self) -> bool {
        false
    }
}

