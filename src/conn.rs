use std::{future::Future, io::{ErrorKind, Result}, net::SocketAddr, pin::Pin, time::{Duration, SystemTime, UNIX_EPOCH}};
use async_trait::async_trait;
use crossterm::event::EventStream;
use futures::{future::pending, FutureExt, StreamExt};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use tokio::{net::{TcpListener, TcpStream, UdpSocket}, select, time::interval};

use crate::{config, display::Display, event::handle_conn_event, player::Player};

#[derive(Clone, Copy)]
pub enum ConnKind {
    Host,
    Client,
    Empty,
}

impl ConnKind {
    pub fn from_args(is_host: bool, is_client: bool) -> Self {
        match (is_host, is_client) {
            (true,  true ) |
            (true,  false) => ConnKind::Host,
            (false, true ) => ConnKind::Client,
            (false, false) => ConnKind::Empty,
        }
    }

    pub fn is_multiplayer(&self) -> bool {
        return !matches!(self, ConnKind::Empty);
    }
}

#[derive(FromPrimitive, ToPrimitive)]
pub enum UdpPacketMode {
    Pos,
}

#[derive(FromPrimitive, ToPrimitive)]
pub enum TcpPacketMode {
    Ping,
    Pong,
    Seeds,
    Place,
    Hold,
}

#[async_trait]
pub trait ConnTrait {
    async fn send_ping(&self) -> Result<()>;
    async fn send_pong(&self, ts_bytes: &[u8; 63]) -> Result<()>;
    async fn send_seeds(&self, p1_seed: u64, p2_seed: u64) -> Result<()>;
    async fn send_place(&self, player: &Player) -> Result<()>;
    async fn send_hold(&self) -> Result<()>;
    async fn send_pos(&self, player: &Player) -> Result<()>;
    fn recv_udp<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(UdpPacketMode, [u8; 63])>> + Send + 'a>>;
    fn recv_tcp<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(TcpPacketMode, [u8; 63])>> + Send + 'a>>;
}

pub struct Conn {
    pub tcp_stream: TcpStream,
    pub udp_socket: UdpSocket,
}

impl Conn {
    async fn tcp_listen(display: &mut Display) -> Result<(TcpStream, SocketAddr)> {
        let mut reader = EventStream::new();
        let tcp_listener = TcpListener::bind(*config::BIND_ADDR).await?;
        loop {
            select! {
                Ok(socket) = tcp_listener.accept() => {
                    return Ok(socket);
                },
                Some(Ok(event)) = reader.next().fuse() => {
                    handle_conn_event(event, display)?;
                },
            }
        }
    }

    async fn tcp_connect(display: &mut Display) -> Result<TcpStream> {
        let mut reader = EventStream::new();
        let mut retry_interval = interval(Duration::from_secs(1));
        loop {
            select! {
                _ = retry_interval.tick() => {
                    if let Ok(stream) = TcpStream::connect(*config::CONN_ADDR).await {
                        return Ok(stream);
                    }
                },
                Some(Ok(event)) = reader.next().fuse() => {
                    handle_conn_event(event, display)?;
                },
            }
        }
    }

    pub async fn establish_connection(conn_kind: ConnKind, display: &mut Display) -> Result<Box<dyn ConnTrait>> {
        match conn_kind {
            ConnKind::Host => {
                let (tcp_stream, peer_addr) = Conn::tcp_listen(display).await?;

                let udp_socket = UdpSocket::bind(*config::BIND_ADDR).await?;
                udp_socket.connect(peer_addr).await?;

                Ok(Box::new(Conn { udp_socket, tcp_stream }))
            },
            ConnKind::Client => {
                let tcp_stream = Conn::tcp_connect(display).await?;

                let udp_socket = UdpSocket::bind(tcp_stream.local_addr().unwrap()).await?;
                udp_socket.connect(*config::CONN_ADDR).await?;

                Ok(Box::new(Conn { udp_socket, tcp_stream }))
            },
            ConnKind::Empty => Ok(Box::new(DummyConn)),
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
    async fn send_ping(&self) -> Result<()> {
        self.send_tcp(
            TcpPacketMode::Ping,
            &SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_le_bytes(),
        ).await?;
        Ok(())
    }

    async fn send_pong(&self, ts_bytes: &[u8; 63]) -> Result<()> {
        self.send_tcp(
            TcpPacketMode::Pong,
            ts_bytes,
        ).await?;
        Ok(())
    }

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
            let mode_bytes: [u8; 1] = buf[0..1].try_into().unwrap();
            let mode = UdpPacketMode::from_u8(u8::from_le_bytes(mode_bytes)).unwrap();
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
                        let mode_bytes: [u8; 1] = buf[0..1].try_into().unwrap();
                        let mode = TcpPacketMode::from_u8(u8::from_le_bytes(mode_bytes)).unwrap();
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
}

pub struct DummyConn;

#[async_trait]
impl ConnTrait for DummyConn {
    async fn send_ping(&self) -> Result<()> {
        Ok(())
    }

    async fn send_pong(&self, _ts_bytes: &[u8; 63]) -> Result<()> {
        Ok(())
    }

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
}

