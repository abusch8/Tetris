use std::{future::Future, io::{ErrorKind, Result}, net::SocketAddr, pin::Pin};
use async_trait::async_trait;
use futures::future::pending;
use tokio::{io::AsyncWriteExt, net::{TcpListener, TcpStream, UdpSocket}};

use crate::{config, game::Game, tetromino::Geometry};

#[async_trait]
pub trait ConnTrait {
    async fn send_seeds(&mut self, p1_sed: u64, p2_seed: u64) -> Result<()>;
    async fn send_pos(&self, game: &mut Game) -> Result<()>;
    async fn send_place(&self) -> Result<()>;
    async fn recv_seeds(&self) -> Result<(u64, u64)>;
    fn recv_pos<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<Geometry>> + Send + 'a>>;
    fn recv_place<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
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
}

#[async_trait]
impl ConnTrait for Conn {
    async fn send_pos(&self, game: &mut Game) -> Result<()> {
        self.udp_socket
            .send(&game.player[0].falling.geometry.to_bytes())
            .await?;
        Ok(())
    }

    async fn send_seeds(&mut self, p1_seed: u64, p2_seed: u64) -> Result<()> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&p1_seed.to_le_bytes());
        buf.extend_from_slice(&p2_seed.to_le_bytes());
        self.tcp_stream.write_all(&buf).await?;
        Ok(())
    }

    fn recv_pos<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<Geometry>> + Send + 'a>> {
        Box::pin(async move {
            let mut buf = [0u8; 41];
            self.udp_socket.recv(&mut buf).await?;
            Ok(Geometry::from_bytes(&buf))
        })
    }

    async fn recv_seeds(&self) -> Result<(u64, u64)> {
        let mut buf = [0u8; 16];
        loop {
            self.tcp_stream.readable().await?;
            match self.tcp_stream.try_read(&mut buf) {
                Ok(_) => {
                    let p1_seed: &[u8; 8] = buf[0..8].try_into().unwrap();
                    let p2_seed: &[u8; 8] = buf[8..16].try_into().unwrap();
                    return Ok((
                        u64::from_le_bytes(*p1_seed),
                        u64::from_le_bytes(*p2_seed),
                    ));
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    async fn send_place(&self) -> Result<()> {
        loop {
            self.tcp_stream.writable().await?;
            match self.tcp_stream.try_write(b"test") {
                Ok(_) => {
                    break;
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }

    fn recv_place<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut buf = [0u8; 41];
            loop {
                self.tcp_stream.readable().await?;
                match self.tcp_stream.try_read(&mut buf) {
                    Ok(_) => {
                        break;
                    }
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }
            Ok(())
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
    async fn send_seeds(&mut self, _p1_seed: u64, _p2_seed: u64) -> Result<()> {
        Ok(())
    }

    async fn send_pos(&self, _game: &mut Game) -> Result<()> {
        Ok(())
    }

    async fn send_place(&self) -> Result<()> {
        Ok(())
    }

    async fn recv_seeds(&self) -> Result<(u64, u64)> {
        Ok((0, 0))
    }

    fn recv_pos<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<Geometry>> + Send + 'a>> {
        Box::pin(pending())
    }

    fn recv_place<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(pending())
    }

    fn is_multiplayer(&self) -> bool {
        false
    }

    fn is_host(&self) -> bool {
        false
    }
}

